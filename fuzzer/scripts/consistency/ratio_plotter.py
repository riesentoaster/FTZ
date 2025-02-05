#!/usr/bin/env python3
import numpy as np
from typing import Dict, List, Tuple
import matplotlib.pyplot as plt
from matplotlib.patches import Patch


def setup_axis(
    ax: plt.Axes, title: str, positions: np.ndarray, all_lengths: List[int]
) -> None:
    """Helper function to set up common axis properties."""
    ax.set_title(title)
    ax.set_xlabel("Input Length")
    ax.set_ylabel("Ratio Value")
    ax.set_xticks(positions)
    ax.set_xticklabels(all_lengths)
    ax.grid(True, alpha=0.3)


def create_violin_plot(
    ax: plt.Axes,
    data: List[List[float]],
    positions: np.ndarray,
    title: str,
    colors: Tuple[str, str, str],
    all_lengths: List[int],
) -> None:
    """Create a violin plot for the given data."""
    violin_parts = ax.violinplot(
        [box for box in data if len(box) > 0],
        positions=[p for p, box in zip(positions, data) if len(box) > 0],
        showmeans=True,
        showmedians=True,
        widths=0.3,
    )

    # Color the violin plots
    for pc in violin_parts["bodies"]:
        pc.set_facecolor(colors[0])
        pc.set_edgecolor(colors[1])
        pc.set_alpha(0.7)
    violin_parts["cmeans"].set_color(colors[1])
    violin_parts["cmedians"].set_color(colors[2])

    setup_axis(ax, title, positions, all_lengths)


def create_box_plot(
    ax: plt.Axes,
    data: List[List[float]],
    positions: np.ndarray,
    title: str,
    colors: Tuple[str, str],
    all_lengths: List[int],
) -> None:
    """Create a box plot for the given data."""
    ax.boxplot(
        data,
        positions=positions,
        patch_artist=True,
        boxprops=dict(facecolor=colors[0], color=colors[1]),
        medianprops=dict(color=colors[1]),
        showfliers=True,
        whis=1.5,
        widths=0.3,
    )
    setup_axis(ax, title, positions, all_lengths)


def create_stats_plot(
    ax: plt.Axes,
    data: List[List[float]],
    positions: np.ndarray,
    title: str,
    color: str,
    all_lengths: List[int],
) -> None:
    """Create statistics plot for the given data."""
    medians = []
    means = []
    for boxes in data:
        if len(boxes) > 0:
            medians.append(np.median(boxes))
            means.append(np.mean(boxes))
        else:
            medians.append(np.nan)
            means.append(np.nan)

    ax.plot(positions, medians, color=color, linestyle="-", label="Median", linewidth=2)
    ax.plot(positions, means, color=color, linestyle="--", label="Mean", linewidth=2)
    setup_axis(ax, title, positions, all_lengths)
    ax.legend()


def create_ratio_plots(
    axes: List[plt.Axes],
    lens: List[int],
    second_ratios: List[float],
    sum_ratios: List[float],
    ratios_by_len: Dict[int, Dict[str, List[float]]],
    data_by_len: Dict[int, List[List[int]]],
) -> None:
    """
    Create violin and box plots for ratio analysis.

    Args:
        axes: List of eight matplotlib axes for plotting
        lens: List of input lengths
        second_ratios: List of second-to-first ratios
        sum_ratios: List of sum-to-first ratios
        ratios_by_len: Dictionary containing ratios by input length
        data_by_len: Raw data dictionary mapping input lengths to their entries
    """
    ax_count1, ax_count2, ax1, ax2, ax3, ax4, ax5, ax6 = axes

    # Create continuous range of lengths
    min_len = min(ratios_by_len.keys())
    max_len = max(ratios_by_len.keys())
    all_lengths = list(range(min_len, max_len + 1))
    positions = np.arange(len(all_lengths)) * 0.35

    # Sample count plots (top row)
    sample_counts = [len(data_by_len.get(l, [])) for l in all_lengths]

    # Left plot: Bar chart of sample counts
    ax_count1.bar(all_lengths, sample_counts, color="gray", alpha=0.7)
    ax_count1.set_title("Number of Samples per Input Length")
    ax_count1.set_xlabel("Input Length")
    ax_count1.set_ylabel("Number of Samples")
    ax_count1.grid(True, alpha=0.3)

    # Right plot: Log scale version
    ax_count2.bar(all_lengths, sample_counts, color="gray", alpha=0.7)
    ax_count2.set_title("Number of Samples per Input Length (Log Scale)")
    ax_count2.set_xlabel("Input Length")
    ax_count2.set_ylabel("Number of Samples")
    ax_count2.set_yscale("log")
    ax_count2.grid(True, alpha=0.3)

    # Prepare data for plots
    second_boxes = [ratios_by_len.get(l, {"second": []})["second"] for l in all_lengths]
    sum_boxes = [ratios_by_len.get(l, {"sum": []})["sum"] for l in all_lengths]

    # Create plots for each ratio type
    plot_configs = [
        {
            "data": second_boxes,
            "title_prefix": "Second/First",
            "colors": ("lightblue", "blue", "darkblue"),
            "axes": (ax1, ax3, ax5),
        },
        {
            "data": sum_boxes,
            "title_prefix": "Sum(Rest)/First",
            "colors": ("lightpink", "red", "darkred"),
            "axes": (ax2, ax4, ax6),
        },
    ]

    for config in plot_configs:
        # Violin plot
        create_violin_plot(
            config["axes"][0],
            config["data"],
            positions,
            f"{config['title_prefix']} Ratio Distribution",
            config["colors"],
            all_lengths,
        )

        # Box plot
        create_box_plot(
            config["axes"][1],
            config["data"],
            positions,
            f"{config['title_prefix']} Ratio Box Plot",
            config["colors"][:2],
            all_lengths,
        )

        # Statistics plot
        create_stats_plot(
            config["axes"][2],
            config["data"],
            positions,
            f"{config['title_prefix']} Ratio Statistics",
            config["colors"][1],
            all_lengths,
        )
