import argparse
import re
import sys
from typing import Any, List, Optional, Tuple
import math

import matplotlib.axis as axis
import matplotlib.dates as mdates
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import numpy as np

pattern = re.compile(
    ".*".join(
        [
            "GLOBAL",
            "run time:\s*(\d+)h-(\d+)m-(\d+)s",
            "clients:\s*(\d+)",
            "corpus:\s*(\d+)",
            "objectives:\s*(\d+)",
            "executions:\s*(\d+)",
            "exec/sec:\s*([\d.]+k?)",
            "coverage_observer:\s*([\d.]+)%",
            "state-observer:\s*([\d.]+)%",
        ]
    )
)


def extract(line: str) -> Optional[Tuple[int, int, int, int, float, float]]:
    m = pattern.search(line)
    if m:
        h, m, s, clients, corpus, objectives, executions, execs_s, coverage, state = m.groups()
        time = int(h) * 3600 + int(m) * 60 + int(s)
        clients = int(clients)
        corpus = int(corpus)
        objectives = int(objectives)
        executions = int(executions)
        if execs_s.endswith("k"):
            exec_s = float(execs_s[:-1]) * 1000
        else:
            exec_s = float(execs_s)
        coverage = float(coverage)
        state = float(state)
        return (time, clients, corpus, objectives, executions, exec_s, coverage, state)
    else:
        if "GLOBAL" in line:
            print(line)


def plot(times: List[int], y: List[Any], ax: axis.Axis, ylabel: str, format_str=None):
    ax.plot(times, y)
    ax.set_xlabel("Time [h]")
    ax.set_ylabel(ylabel)
    if format_str:
        ax.yaxis.set_major_formatter(ticker.FormatStrFormatter(format_str))
    xaxis_fmt = (
        (lambda x, pos: f"{x/3600:.0f}")
        if max(times) > 6 * 3600
        else (lambda x, pos: f"{x/3600:.1f}")
    )
    ax.xaxis.set_major_formatter(ticker.FuncFormatter(xaxis_fmt))
    if max(times) > 6 * 3600:
        xaxis_interval = (max(times) // 10 // 1800) * 1800
    else:
        xaxis_interval = 1800

    ax.xaxis.set_major_locator(ticker.MultipleLocator(xaxis_interval))
    ax.grid()


def main():
    parser = argparse.ArgumentParser("Create plots from a multimonitor output file.")
    parser.add_argument(
        "input",
        help="Path to the input file. Anything before the last dot will be used as the title and output filename.",
    )
    args = parser.parse_args()

    with open(args.input) as f:
        lines = f.readlines()
        lines = [data for line in lines if (data := extract(line))]
        times, clients, corpus, objectives, executions, exec_s, coverage, state = zip(*lines)

    configs = [
        {"y": clients, "ylabel": "Clients [count]"},
        {"y": corpus, "ylabel": "Corpus [count]"},
        {"y": objectives, "ylabel": "Objectives [count]"},
        {"y": executions, "ylabel": "Executions [count]"},
        {"y": exec_s, "ylabel": "Executions/s [1/s]", "format_str": "%.3f"},
        {"y": coverage, "ylabel": "Coverage [%]", "format_str": "%.3f"},
        {"y": state, "ylabel": "State [%]", "format_str": "%.3f"},
    ]

    plt_height = math.ceil(math.sqrt(len(configs)))
    plt_width = math.ceil(len(configs) / plt_height)

    fig, axes = plt.subplots(
        plt_height, plt_width, figsize=(7 * plt_width, 7 * plt_height + 1)
    )
    axes = axes.flatten()

    for config, ax in zip(configs, list(axes)):
        plot(**config, ax=ax, times=times)

    base = ".".join(args.input.split(".")[:-1]) if "." in args.input else args.input
    fig.suptitle(base.split("/")[-1])
    fig.tight_layout()
    fig.savefig(f"{base}.png", dpi=200)


if __name__ == "__main__":
    main()
