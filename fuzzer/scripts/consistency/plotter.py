"""Main plotting functionality for consistency analysis."""

import matplotlib.pyplot as plt
import numpy as np
import os
from typing import Optional, Tuple

from .data_extractor import (
    extract_data,
    calculate_global_ranges,
    calculate_error_ratios,
)
from .distribution_plotter import create_distribution_plots
from .ratio_plotter import create_ratio_plots, create_box_plot, create_violin_plot


def create_ratio_figure(data_by_len, base_name):
    """Create a figure with ratio plots."""
    fig, axes = plt.subplots(
        4, 2, figsize=(20, 40)  # Increased width from 15 to 20 inches
    )
    lens, second_ratios, sum_ratios, ratios_by_len = calculate_error_ratios(data_by_len)
    create_ratio_plots(
        axes.flatten(),
        lens,
        second_ratios,
        sum_ratios,
        ratios_by_len,
        data_by_len,
    )

    min_len = min(ratios_by_len.keys())
    max_len = max(ratios_by_len.keys())
    all_lengths = list(range(min_len, max_len + 1))

    sum_boxes = [ratios_by_len.get(l, {"sum": []})["sum"] for l in all_lengths]
    positions = np.arange(len(all_lengths)) * 0.35

    boxplot_fig, boxplot_ax = plt.subplots(1, 1, figsize=(10, 10))

    create_box_plot(
        boxplot_ax,
        sum_boxes,
        positions,
        "Sum(Rest)/First Ratio Box Plot",
        ("lightpink", "red", "darkred"),
        all_lengths,
    )
    boxplot_output = base_name + "-boxplot.svg"
    boxplot_ax.set_title(None)
    boxplot_fig.tight_layout()
    boxplot_fig.savefig(boxplot_output, bbox_inches="tight")
    plt.close(boxplot_fig)
    print(f"Separate boxplot saved as '{boxplot_output}'")

    violin_fig, violin_ax = plt.subplots(1, 1, figsize=(10, 10))

    create_violin_plot(
        violin_ax,
        sum_boxes,
        positions,
        f"Sum(Rest)/First Ratio Distribution",
        ("lightpink", "red", "darkred"),
        all_lengths,
    )
    violin_output = base_name + "-violin.svg"
    violin_ax.set_title(None)
    violin_fig.tight_layout()
    violin_fig.savefig(violin_output, bbox_inches="tight")
    plt.close(violin_fig)
    print(f"Separate violin plot saved as '{violin_output}'")

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


def process_log_file(
    logfile: str,
    print_counts: bool = False,
    plot_ratios: bool = True,
    plot_distributions: bool = True,
) -> Tuple[int, Optional[str], Optional[str]]:
    """
    Process a log file and generate plots.

    Args:
        logfile: Path to the log file to process
        print_counts: Whether to print length counts
        plot_ratios: Whether to generate ratio plots
        plot_distributions: Whether to generate distribution plots

    Returns:
        Tuple containing:
        - Exit code (0 for success, 1 for failure)
        - Path to ratio plots file (if generated, None otherwise)
        - Path to distribution plots file (if generated, None otherwise)
    """
    # Extract and process data
    data_by_len, unfixed_count = extract_data(logfile)
    if not data_by_len:
        print("No matching log entries found!")
        return 1, None, None

    # Calculate global ranges for consistent axes
    global_ranges = calculate_global_ranges(data_by_len)
    ratio_output = None
    dist_output = None

    # Create and save ratio plots if requested
    if plot_ratios:
        base_name = os.path.splitext(logfile)[0]
        ratio_fig = create_ratio_figure(data_by_len, base_name)
        ratio_output = base_name + "-ratios.svg"
        ratio_fig.tight_layout()
        ratio_fig.savefig(ratio_output, bbox_inches="tight")
        plt.close(ratio_fig)
        print(f"Ratio plots saved as '{ratio_output}'")

    # Create and save distribution plots if requested
    if plot_distributions:
        dist_fig, unique_lengths, length_counts = create_distribution_figure(
            data_by_len, global_ranges
        )
        dist_output = os.path.splitext(logfile)[0] + "-distributions.svg"
        dist_fig.tight_layout()
        dist_fig.savefig(dist_output, bbox_inches="tight")
        plt.close(dist_fig)
        print(f"Distribution plots saved as '{dist_output}'")

        if print_counts:
            print("\nList Length Counts (Combined):")
            for length, count in zip(unique_lengths, length_counts):
                print(f"Length {length}: {count} entries")

    # Print statistics
    all_data = [item for sublist in data_by_len.values() for item in sublist]
    print(f"Number of unfixable inputs: {unfixed_count}")
    print(f"Max number of replay to stable: {max(max(x) for x in all_data)}")

    return 0, ratio_output, dist_output
