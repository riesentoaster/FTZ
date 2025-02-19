#!/usr/bin/env python3

import json
import sys
import argparse
from pathlib import Path
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter, ScalarFormatter
import numpy as np

global_config = {
    "coverage-observer": {
        "factor": 100,
        "ylabel": "Code Coverage [%]",
        "print-minor-ticks": True,
        "ymin": 6,
    },
    "state-map-observer": {
        "factor": 100,
        "ylabel": "State Coverage [%]",
    },
    "state-diff-map-observer": {
        "factor": 100,
        "ylabel": "State-Diff Coverage [%]",
        "ymin": 0.01,
    },
    "all_other_to_most_rolling_avg": {"ylabel": "Consistency Ratio", "ymin": 0.0001},
    "corpus": {"ylabel": "Corpus Size"},
    "exec_sec": {"ylabel": "Execution Speed (executions/s)"},
    "executions": {"ylabel": "Executions"},
    "free_memory": {"ylabel": "Free Memory (bytes)", "print-minor-ticks": True},
    "input_len": {"ylabel": "Average Input Length"},
    "objectives": {"ylabel": "Objectives"},
    "run_time": {"ylabel": "Run Time (seconds)"},
    "second_to_most_rolling_avg": {
        "ylabel": "Second to Most Rolling Avg",
        "factor": 100,
    },
    "uncaptured-inconsistent-rate": {
        "ylabel": "Uncaptured Inconsistent Rate [%]",
        "factor": 100,
        "print-minor-ticks": True,
    },
    "consistency-caused-replay-per-input": {
        "ylabel": "Consistency Caused Replay Per Input [%]",
        "print-minor-ticks": True,
    },
    "consistency-caused-replay-per-input-success": {
        "ylabel": "Consistency Caused Replay Per Input Success [%]",
        "print-minor-ticks": True,
    },
}


def parse_entry(data):
    result = {}
    for key, value in data.items():
        if isinstance(value, dict) and len(value) == 1:
            # Handle nested single-value dicts like {"Float": 0.5} or {"Percent": 0.3}
            nested_key, nested_value = next(iter(value.items()))
            result[key] = nested_value
        else:
            result[key] = value
    return result


def parse_file(path):
    entries = []
    with open(path) as f:
        for line in f:
            if line := line.strip():
                try:
                    entries.append(parse_entry(json.loads(line)))
                except Exception as e:
                    print(f"Warning: {e}", file=sys.stderr)
    return entries


def plot(file_data, x_axis, key, limit_range, out_dir):
    config = global_config[key]
    fig, ax = plt.subplots(figsize=(12, 6))

    # Filter to only files that have the key
    valid_files = {
        path: entries
        for path, entries in file_data.items()
        if any(key in entry for entry in entries)
    }

    if not valid_files:
        print(f"Error: No files contain the metric '{key}'", file=sys.stderr)
        return

    for path, entries in valid_files.items():
        x_values = [entry.get(x_axis, 0) for entry in entries]
        coverage = [entry.get(key, 0) for entry in entries]

        if "factor" in config:
            coverage = [x * config["factor"] for x in coverage]

        ax.plot(
            x_values, coverage, label=".".join(path.name.split(".")[:-1]), alpha=0.7
        )

    if limit_range:
        min_x_range = min(
            max(entry.get(x_axis, 0) for entry in entries)
            for entries in valid_files.values()
        )
        ax.set_xlim(0, min_x_range)

    ax.set_xlabel("Run Time (seconds)" if x_axis == "run_time" else "Executions")
    ax.set_ylabel(config["ylabel"])
    ax.semilogy()

    # Format x-axis to show full numbers
    ax.xaxis.set_major_formatter(ScalarFormatter(useOffset=False))
    ax.ticklabel_format(style="plain", axis="x")

    # Format y-axis ticks with appropriate decimal places based on value
    def format_tick(y, pos):
        if y == 0:
            return "0"
        decimal_places = int(np.maximum(-np.log10(y), 0))
        return f"{{:.{decimal_places}f}}".format(y)

    ax.yaxis.set_major_formatter(FuncFormatter(format_tick))

    if config.get("print-minor-ticks", False):
        ax.yaxis.set_minor_formatter(FuncFormatter(format_tick))

    if "ymin" in config:
        ax.set_ylim(bottom=config["ymin"])

    ax.legend()
    ax.grid(which="both")
    output = Path(out_dir) / f"{key.replace('_', '-')}-by-{x_axis}-plot.svg"
    fig.tight_layout()
    output.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(output)
    plt.close(fig)
    print(f"Plot saved as {output}")


def main():
    parser = argparse.ArgumentParser(
        description="Parse and plot metrics from JSON log files"
    )
    parser.add_argument(
        "files", nargs="+", type=Path, help="One or more metrics files to parse"
    )
    parser.add_argument(
        "--x-axis",
        choices=["run_time", "executions", "all"],
        default="run_time",
        help="Value to use for x-axis (default: run_time). Use 'all' to generate plots for all x-axis options",
    )
    parser.add_argument(
        "--key",
        choices=list(global_config.keys()) + ["all"],
        default="coverage-observer",
        help="Value to use for y-axis (default: coverage-observer). Use 'all' to generate plots for all metrics",
    )
    parser.add_argument(
        "--limit-range",
        action="store_true",
        help="Limit x-axis range to the minimum range across all input files",
    )
    parser.add_argument(
        "--out-dir",
        type=str,
        default=".",
        help="Directory where plot files will be saved (default: current directory)",
    )
    args = parser.parse_args()

    file_data = {}
    for path in args.files:
        if path.exists():
            file_data[path] = parse_file(path)
        else:
            print(f"Error: {path} does not exist", file=sys.stderr)
            exit(1)

    x_axis_options = (
        ["run_time", "executions"] if args.x_axis == "all" else [args.x_axis]
    )
    key_options = list(global_config.keys()) if args.key == "all" else [args.key]

    for x_axis in x_axis_options:
        for key in key_options:
            plot(file_data, x_axis, key, args.limit_range, args.out_dir)


if __name__ == "__main__":
    main()
