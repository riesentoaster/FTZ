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
        "ylabel": "Block Coverage [%]",
        "print-minor-ticks": True,
    },
    "state-map-observer": {
        "factor": 100,
        "ylabel": "State Coverage [%]",
    },
    "state-diff-map-observer": {
        "factor": 100,
        "ylabel": "State-Diff Coverage [%]",
    },
    "all_other_to_most_rolling_avg": {"ylabel": "Consistency Ratio"},
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


def plot(file_data, x_axis, key, limit_range):
    config = global_config[key]
    fig, ax = plt.subplots(figsize=(12, 8))

    # Filter to only files that have the key
    valid_files = {
        path: entries
        for path, entries in file_data.items()
        if any(key in entry for entry in entries)
    }

    if not valid_files:
        print(f"Error: No files contain the metric '{key}'", file=sys.stderr)
        exit(1)

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

    ax.legend()
    ax.grid(which="both")
    output = f"{key.replace('_', '-')}-plot.svg"
    fig.tight_layout()
    fig.savefig(output)
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
        choices=["run_time", "executions"],
        default="run_time",
        help="Value to use for x-axis (default: run_time)",
    )
    parser.add_argument(
        "--key",
        choices=global_config.keys(),
        default="coverage-observer",
        help="Value to use for y-axis (default: coverage-observer)",
    )
    parser.add_argument(
        "--limit-range",
        action="store_true",
        help="Limit x-axis range to the minimum range across all input files",
    )
    args = parser.parse_args()

    file_data = {}
    for path in args.files:
        if path.exists():
            file_data[path] = parse_file(path)
        else:
            print(f"Error: {path} does not exist", file=sys.stderr)
            exit(1)

    plot(file_data, args.x_axis, args.key, args.limit_range)


if __name__ == "__main__":
    main()
