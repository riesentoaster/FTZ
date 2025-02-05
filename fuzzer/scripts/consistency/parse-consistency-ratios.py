#!/usr/bin/env python3
import argparse
import matplotlib.pyplot as plt
import os
from .data_extractor import (
    extract_data,
    calculate_global_ranges,
    calculate_error_ratios,
)
from .distribution_plotter import create_distribution_plots
from .ratio_plotter import create_ratio_plots


def create_ratio_figure(data_by_len):
    """Create a figure with ratio plots."""
    fig = plt.figure(figsize=(15, 5))
    axes = [plt.subplot(1, 3, i + 1) for i in range(3)]
    lens, second_ratios, sum_ratios, ratios_by_len = calculate_error_ratios(data_by_len)
    create_ratio_plots(axes, lens, second_ratios, sum_ratios, ratios_by_len)
    return fig


def create_distribution_figure(data_by_len, global_ranges):
    """Create a figure with all distribution plots."""
    n_rows = len(data_by_len) + 1  # +1 for combined plots
    fig = plt.figure(figsize=(15, 5 * n_rows))

    # Create combined distribution plots
    all_data = [item for sublist in data_by_len.values() for item in sublist]
    axes_combined = [plt.subplot(n_rows, 3, i + 1) for i in range(3)]
    unique_lengths, length_counts = create_distribution_plots(
        all_data, axes_combined, global_ranges, "Combined: "
    )

    # Create per-length distribution plots
    for idx, (input_len, len_data) in enumerate(sorted(data_by_len.items())):
        row = idx + 1
        axes = [plt.subplot(n_rows, 3, 3 * row + i + 1) for i in range(3)]
        create_distribution_plots(
            len_data, axes, global_ranges, f"Length {input_len}: "
        )

    return fig, unique_lengths, length_counts


def main():
    parser = argparse.ArgumentParser(
        description="Parse and plot consistency ratios from log file."
    )
    parser.add_argument("logfile", help="Input log file to process")
    parser.add_argument(
        "--print-counts", action="store_true", help="Print list length counts"
    )
    args = parser.parse_args()

    # Extract and process data
    data_by_len, unfixed_count = extract_data(args.logfile)
    if not data_by_len:
        print("No matching log entries found!")
        return 1

    # Calculate global ranges for consistent axes
    global_ranges = calculate_global_ranges(data_by_len)

    # Create and save ratio plots
    ratio_fig = create_ratio_figure(data_by_len)
    ratio_output = os.path.splitext(args.logfile)[0] + "-ratios.svg"
    ratio_fig.tight_layout()
    ratio_fig.savefig(ratio_output, bbox_inches="tight")
    plt.close(ratio_fig)
    print(f"Ratio plots saved as '{ratio_output}'")

    # Create and save distribution plots
    dist_fig, unique_lengths, length_counts = create_distribution_figure(
        data_by_len, global_ranges
    )
    dist_output = os.path.splitext(args.logfile)[0] + "-distributions.svg"
    dist_fig.tight_layout()
    dist_fig.savefig(dist_output, bbox_inches="tight")
    plt.close(dist_fig)
    print(f"Distribution plots saved as '{dist_output}'")

    # Print statistics
    if args.print_counts:
        print("\nList Length Counts (Combined):")
        for length, count in zip(unique_lengths, length_counts):
            print(f"Length {length}: {count} entries")

    all_data = [item for sublist in data_by_len.values() for item in sublist]
    print(f"Number of unfixable inputs: {unfixed_count}")
    print(f"Max number of replay to stable: {max(max(x) for x in all_data)}")

    return 0


if __name__ == "__main__":
    exit(main())
