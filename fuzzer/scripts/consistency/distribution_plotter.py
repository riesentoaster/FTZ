#!/usr/bin/env python3
import numpy as np
from typing import Dict, List, Tuple
import matplotlib.pyplot as plt


def create_distribution_plots(
    data: List[List[int]],
    axes: List[plt.Axes],
    global_ranges: Dict[str, Tuple[int, int]],
    title_prefix: str = "",
) -> Tuple[np.ndarray, np.ndarray]:
    """
    Create distribution plots for the given data.

    Args:
        data: List of data entries
        axes: List of three matplotlib axes for plotting
        global_ranges: Dictionary containing plot ranges
        title_prefix: Optional prefix for plot titles

    Returns:
        Tuple of unique lengths and their counts
    """
    ax1, ax2, ax3 = axes

    # Transform and flatten data
    relative_data = [[x[0] - y for y in x] for x in data if x]
    flattened_data = [item for sublist in relative_data for item in sublist]

    # Plot 1: All relative ratios
    ratios, counts = np.unique(flattened_data, return_counts=True)
    ax1.bar(ratios, counts)
    ax1.set_title(f"{title_prefix}Distribution of Relative Consistency Ratios")
    ax1.set_xlabel("Difference from First Value")
    ax1.set_ylabel("Frequency (log scale)")
    ax1.set_yscale("log")
    ax1.set_xlim(global_ranges["ratio"])
    ax1.grid(True, alpha=0.3)

    # Plot 2: List lengths
    lengths, counts = np.unique([len(x) for x in data], return_counts=True)
    ax2.bar(lengths, counts)
    ax2.set_title(f"{title_prefix}Distribution of Number of Values per Entry")
    ax2.set_xlabel("Number of Values")
    ax2.set_ylabel("Frequency (log scale)")
    ax2.set_yscale("log")
    ax2.set_xlim(global_ranges["length"])
    ax2.grid(True, alpha=0.3)

    # Plot 3: Filtered ratios (lists with ≥2 entries)
    filtered_data = [
        item for sublist in relative_data for item in sublist if len(sublist) >= 2
    ]
    ratios, counts = np.unique(filtered_data, return_counts=True)
    ax3.bar(ratios, counts)
    ax3.set_title(
        f"{title_prefix}Distribution of Relative Consistency Ratios\n(Lists with ≥2 Entries)"
    )
    ax3.set_xlabel("Difference from First Value")
    ax3.set_ylabel("Frequency (log scale)")
    ax3.set_yscale("log")
    ax3.set_xlim(global_ranges["filtered"])
    ax3.grid(True, alpha=0.3)

    return lengths, counts
