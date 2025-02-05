#!/usr/bin/env python3
import numpy as np
from typing import Dict, List
import matplotlib.pyplot as plt
from matplotlib.patches import Patch


def create_ratio_plots(
    axes: List[plt.Axes],
    lens: List[int],
    second_ratios: List[float],
    sum_ratios: List[float],
    ratios_by_len: Dict[int, Dict[str, List[float]]],
) -> None:
    """
    Create scatter and box plots for ratio analysis.

    Args:
        axes: List of three matplotlib axes for plotting
        lens: List of input lengths
        second_ratios: List of second-to-first ratios
        sum_ratios: List of sum-to-first ratios
        ratios_by_len: Dictionary containing ratios by input length
    """
    ax1, ax2, ax3 = axes

    # Scatter plots
    ax1.scatter(lens, second_ratios, alpha=0.5, color="blue")
    ax1.set_title("Second/First Ratio vs Input Length")
    ax1.set_xlabel("Input Length")
    ax1.set_ylabel("Second/First Ratio")
    ax1.grid(True, alpha=0.3)

    ax2.scatter(lens, sum_ratios, alpha=0.5, color="red")
    ax2.set_title("Sum(Rest)/First Ratio vs Input Length")
    ax2.set_xlabel("Input Length")
    ax2.set_ylabel("Sum(Rest)/First Ratio")
    ax2.grid(True, alpha=0.3)

    # Box plots
    lengths = sorted(ratios_by_len.keys())
    positions = np.arange(len(lengths)) * 3

    colors = {"second": ("lightblue", "blue"), "sum": ("lightpink", "red")}

    for i, (key, (fill_color, edge_color)) in enumerate(colors.items()):
        boxes = [ratios_by_len[l][key] for l in lengths]
        ax3.boxplot(
            boxes,
            positions=positions + i,
            patch_artist=True,
            boxprops=dict(facecolor=fill_color, color=edge_color),
            medianprops=dict(color=edge_color),
            labels=[""] * len(lengths),
        )

    ax3.set_xticks(positions + 0.5)
    ax3.set_xticklabels(lengths)
    ax3.set_title("Distribution of Ratios by Input Length")
    ax3.set_xlabel("Input Length")
    ax3.set_ylabel("Ratio Value")
    ax3.grid(True, alpha=0.3)

    # Legend
    legend_elements = [
        Patch(facecolor=fill_color, edgecolor=edge_color, label=key.title())
        for key, (fill_color, edge_color) in colors.items()
    ]
    ax3.legend(handles=legend_elements, loc="upper right")
