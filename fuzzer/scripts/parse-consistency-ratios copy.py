#!/usr/bin/env python3
import re, sys
import matplotlib.pyplot as plt
import numpy as np
from scipy import stats
from scipy import optimize
import os
import argparse
from matplotlib.patches import Patch

parser = argparse.ArgumentParser(
    description="Parse and plot consistency ratios from log file."
)
parser.add_argument("logfile", help="Input log file to process")
parser.add_argument(
    "--print-counts", action="store_true", help="Print list length counts"
)
args = parser.parse_args()

# Get the base filename without extension and create output filename
output_filename = os.path.splitext(args.logfile)[0] + "-distributions.svg"

# Extract numbers from log lines and create DataFrame
data = []
data_by_len = {}  # Dictionary to store data by input length
pattern = r".+consistency_ratios for input of len (\d+):\s+([\d,\s]+)\n"
unfixed_count = 0
with open(args.logfile) as f:
    for line in f:
        if re.search(pattern, line):
            match = re.search(pattern, line)
            input_len = int(match.group(1))
            values = [int(i) for i in match.group(2).split(",")]
            data.append(values)
            if input_len not in data_by_len:
                data_by_len[input_len] = []
            data_by_len[input_len].append(values)
        elif "Input inconsistent" in line:
            unfixed_count += 1
        elif "tries, still unstable" in line:
            print(line[:-1])

if not data:
    print("No matching log entries found!")
    sys.exit(1)

# Calculate global ranges for x-axes
all_diffs = [x[0] - y for x in data for y in x]
filtered_diffs = [x[0] - y for x in data if len(x) >= 2 for y in x]
global_ranges = {
    "ratio": (min(all_diffs), max(all_diffs)),
    "length": (min(len(x) for x in data), max(len(x) for x in data)),
    "filtered": (
        (min(filtered_diffs), max(filtered_diffs))
        if filtered_diffs
        else (min(all_diffs), max(all_diffs))
    ),
}


# Function to create plots for a given dataset
def create_plots(data, axes, title_prefix=""):
    ax1, ax2, ax3 = axes
    # Transform data to get differences from first entry in each sublist
    relative_data = list(
        map(
            lambda sublist: [sublist[0] - item for item in sublist],
            filter(lambda sublist: len(sublist) > 0, data),
        )
    )

    flattened_relative_data = [item for sublist in relative_data for item in sublist]

    # Get list lengths
    list_lengths = [len(sublist) for sublist in data]

    # First subplot: Distribution of relative ratio values
    unique_ratios, ratio_counts = np.unique(flattened_relative_data, return_counts=True)
    ax1.bar(unique_ratios, ratio_counts)
    ax1.set_title(f"{title_prefix}Distribution of Relative Consistency Ratios")
    ax1.set_xlabel("Difference from First Value")
    ax1.set_ylabel("Frequency (log scale)")
    ax1.grid(True, alpha=0.3)
    ax1.set_yscale("log")
    ax1.set_xlim(global_ranges["ratio"])

    # Second subplot: Distribution of list lengths
    unique_lengths, length_counts = np.unique(list_lengths, return_counts=True)
    ax2.bar(unique_lengths, length_counts)
    ax2.set_title(f"{title_prefix}Distribution of Number of Values per Entry")
    ax2.set_xlabel("Number of Values")
    ax2.set_ylabel("Frequency (log scale)")
    ax2.grid(True, alpha=0.3)
    ax2.set_yscale("log")
    ax2.set_xlim(global_ranges["length"])

    # Third subplot: Distribution of ratio values for lists with at least 2 entries
    filtered_relative_data = [sublist for sublist in relative_data if len(sublist) >= 2]
    flattened_filtered_relative_data = [
        item for sublist in filtered_relative_data for item in sublist
    ]
    unique_filtered_ratios, filtered_ratio_counts = np.unique(
        flattened_filtered_relative_data, return_counts=True
    )
    ax3.bar(unique_filtered_ratios, filtered_ratio_counts)
    ax3.set_title(
        f"{title_prefix}Distribution of Relative Consistency Ratios\n(Lists with ≥2 Entries)"
    )
    ax3.set_xlabel("Difference from First Value")
    ax3.set_ylabel("Frequency (log scale)")
    ax3.grid(True, alpha=0.3)
    ax3.set_yscale("log")
    ax3.set_xlim(global_ranges["filtered"])

    return unique_lengths, length_counts


def calculate_error_ratios(data_by_len):
    lens = []
    second_to_first_ratios = []
    sum_to_first_ratios = []
    # For box plots
    ratios_by_len = {}  # Dictionary to store ratios for each input length

    for input_len, len_data in sorted(data_by_len.items()):
        second_ratios_for_len = []
        sum_ratios_for_len = []
        for sublist in len_data:
            if len(sublist) >= 2:  # Need at least 2 elements for the ratios
                lens.append(input_len)
                # Calculate ratio of second element to first
                second_ratio = sublist[1] / sublist[0]
                second_to_first_ratios.append(second_ratio)
                second_ratios_for_len.append(second_ratio)
                # Calculate ratio of sum of rest to first
                sum_rest = sum(sublist[1:])
                sum_ratio = sum_rest / sublist[0]
                sum_to_first_ratios.append(sum_ratio)
                sum_ratios_for_len.append(sum_ratio)

        if second_ratios_for_len:  # Only add if we have data for this length
            ratios_by_len[input_len] = {
                "second": second_ratios_for_len,
                "sum": sum_ratios_for_len,
            }

    return lens, second_to_first_ratios, sum_to_first_ratios, ratios_by_len


# Calculate number of rows needed for the grid
n_input_lens = len(data_by_len)
n_rows = n_input_lens + 2  # +2 for the combined plot and error ratios plots

# Create a figure with subplots arranged in a grid
fig = plt.figure(figsize=(15, 5 * n_rows))

# Create error ratios plots at the top
lens, second_ratios, sum_ratios, ratios_by_len = calculate_error_ratios(data_by_len)

# First error ratio plot (Second/First)
ax_ratio1 = plt.subplot(n_rows, 3, 1)
ax_ratio1.scatter(lens, second_ratios, alpha=0.5, color="blue")
ax_ratio1.set_title("Second/First Ratio vs Input Length")
ax_ratio1.set_xlabel("Input Length")
ax_ratio1.set_ylabel("Second/First Ratio")
ax_ratio1.grid(True, alpha=0.3)

# Second error ratio plot (Sum/First)
ax_ratio2 = plt.subplot(n_rows, 3, 2)
ax_ratio2.scatter(lens, sum_ratios, alpha=0.5, color="red")
ax_ratio2.set_title("Sum(Rest)/First Ratio vs Input Length")
ax_ratio2.set_xlabel("Input Length")
ax_ratio2.set_ylabel("Sum(Rest)/First Ratio")
ax_ratio2.grid(True, alpha=0.3)

# Box plots for both ratios
ax_ratio3 = plt.subplot(n_rows, 3, 3)
lengths = sorted(ratios_by_len.keys())

# Prepare data for box plots
second_boxes = [ratios_by_len[l]["second"] for l in lengths]
sum_boxes = [ratios_by_len[l]["sum"] for l in lengths]

# Create positions for box plots
positions = np.arange(len(lengths)) * 3

# Define colors
second_color = "lightblue"
sum_color = "lightpink"
second_edge_color = "blue"
sum_edge_color = "red"

# Create box plots
bp1 = ax_ratio3.boxplot(
    second_boxes,
    positions=positions,
    tick_labels=[""] * len(lengths),  # Empty labels for first set
    patch_artist=True,
    boxprops=dict(facecolor=second_color, color=second_edge_color),
    medianprops=dict(color=second_edge_color),
)
bp2 = ax_ratio3.boxplot(
    sum_boxes,
    positions=positions + 1,
    tick_labels=[""] * len(lengths),  # Empty labels for second set
    patch_artist=True,
    boxprops=dict(facecolor=sum_color, color=sum_edge_color),
    medianprops=dict(color=sum_edge_color),
)

# Set custom tick positions and labels
ax_ratio3.set_xticks(positions + 0.5)  # Center between each pair of boxes
ax_ratio3.set_xticklabels(lengths)

ax_ratio3.set_title("Distribution of Ratios by Input Length")
ax_ratio3.set_xlabel("Input Length")
ax_ratio3.set_ylabel("Ratio Value")
ax_ratio3.grid(True, alpha=0.3)

# Create custom legend handles
legend_elements = [
    Patch(facecolor=second_color, edgecolor=second_edge_color, label="Second/First"),
    Patch(facecolor=sum_color, edgecolor=sum_edge_color, label="Sum/First"),
]
ax_ratio3.legend(handles=legend_elements, loc="upper right")

# Create combined plot below error ratios
axes_combined = [plt.subplot(n_rows, 3, i + 4) for i in range(3)]  # Start from row 2
unique_lengths, length_counts = create_plots(data, axes_combined, "Combined: ")

# Print list length counts for combined data
if args.print_counts:
    print("\nList Length Counts (Combined):")
    for length, count in zip(unique_lengths, length_counts):
        print(f"Length {length}: {count} entries")

# Create separate plots for each input length
for idx, (input_len, len_data) in enumerate(sorted(data_by_len.items())):
    row = idx + 2  # Start from third row (after error ratios and combined plots)
    axes = [plt.subplot(n_rows, 3, 3 * row + 1) for i in range(3)]

    # Create plots for this input length
    unique_lengths, length_counts = create_plots(
        len_data, axes, f"Length {input_len}: "
    )

    # Print list length counts for this input length
    if args.print_counts:
        print(f"\nList Length Counts (Input Length {input_len}):")
        for length, count in zip(unique_lengths, length_counts):
            print(f"Length {length}: {count} entries")

# Adjust layout and save
plt.tight_layout()
plt.savefig(output_filename, bbox_inches="tight")
plt.close()

print(f"All distribution plots saved as '{output_filename}'")
print(f"Number of unfixable inputs: {unfixed_count}")
print(
    f"Max number of replay to stable: {max(item for sublist in data for item in sublist)}"
)
