import argparse, re, matplotlib.pyplot as plt, os
import numpy as np
from collections import defaultdict


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input_file", help="Input file to process")
    parser.add_argument(
        "--type", choices=["state", "coverage"], help="Type of consistency to analyze"
    )
    args = parser.parse_args()

    base_output_file = os.path.splitext(args.input_file)[0]
    pie_output_file = f"{base_output_file}-observer-ratios.svg"
    bar_output_file = f"{base_output_file}-observer-ratios-by-len.svg"

    # Dictionary to store ratios by input length
    ratios_by_len = defaultdict(list)

    try:
        with open(args.input_file) as f:
            for line in f:
                if args.type:
                    # Parse the simplified format for single observer
                    if match := re.search(
                        r"consistency_ratios for input of len (\d+): ([\d,\s]+)",
                        line,
                    ):
                        input_len = int(match.groups()[0])
                        values = [int(x.strip()) for x in match.groups()[1].split(",")]
                        stable_count = values[0]
                        unstable_count = sum(values[1:])
                        total = stable_count + unstable_count
                        if total > 0:  # Avoid division by zero
                            # Format ratios to match the existing format:
                            # [both_unstable, coverage_unstable, state_unstable, both_stable]
                            if args.type == "coverage":
                                ratios = [
                                    0,
                                    unstable_count / total,
                                    0,
                                    stable_count / total,
                                ]
                            else:  # state
                                ratios = [
                                    0,
                                    0,
                                    unstable_count / total,
                                    stable_count / total,
                                ]
                            ratios_by_len[input_len].append(ratios)
                else:
                    # Parse the original format for both observers
                    if match := re.search(
                        r"Observer correctness stats for input of len (\d+): both wrong: (\d+), first wrong/second right: (\d+), first right/second wrong: (\d+), both right: (\d+)",
                        line,
                    ):
                        input_len = int(match.groups()[0])
                        numbers = [int(x) for x in match.groups()[1:]]
                        total = sum(numbers)
                        if total > 0:  # Avoid division by zero
                            ratios = [n / total for n in numbers]
                            ratios_by_len[input_len].append(ratios)

            if ratios_by_len:
                # Calculate overall average for pie chart
                all_ratios = [
                    ratio
                    for ratios_list in ratios_by_len.values()
                    for ratio in ratios_list
                ]
                avg_ratios = np.mean(all_ratios, axis=0)

                # Determine which types of instability are present
                has_both_unstable = avg_ratios[0] > 0
                has_coverage_unstable = avg_ratios[1] > 0
                has_state_unstable = avg_ratios[2] > 0
                has_both_stable = avg_ratios[3] > 0

                # Set labels and colors based on which types appear
                if has_coverage_unstable and has_state_unstable:
                    labels = [
                        "Both Unstable",
                        "Coverage Unstable",
                        "State Unstable",
                        "Both Stable",
                    ]
                    colors = ["#cc0000", "#0066cc", "#cc6600", "#006600"]
                elif has_coverage_unstable:
                    # Combine "Both Unstable" and "Coverage Unstable" into just "Unstable"
                    avg_ratios = [
                        avg_ratios[0] + avg_ratios[1],
                        0,
                        0,
                        avg_ratios[2] + avg_ratios[3],
                    ]
                    labels = ["Unstable", "", "", "Stable"]
                    colors = ["#cc0000", "#ffffff", "#ffffff", "#006600"]
                elif has_state_unstable:
                    # Combine "Both Unstable" and "State Unstable" into just "Unstable"
                    avg_ratios = [
                        avg_ratios[0] + avg_ratios[2],
                        0,
                        0,
                        avg_ratios[1] + avg_ratios[3],
                    ]
                    labels = ["Unstable", "", "", "Stable"]
                    colors = ["#cc0000", "#ffffff", "#ffffff", "#006600"]

                # Create pie chart
                plt.figure(figsize=(10, 8))
                plt.pie(
                    avg_ratios,
                    labels=labels,
                    colors=colors,
                    autopct=lambda pct: f"{pct:.5f}%" if pct > 0 else "",
                )
                plt.axis("equal")
                plt.savefig(pie_output_file)
                plt.close()
                print(f"Pie chart saved to {pie_output_file}")

                # Create stacked bar chart
                plt.figure(figsize=(10, 6))
                input_lens = sorted(ratios_by_len.keys())
                avg_ratios_by_len = {
                    length: np.mean(ratios, axis=0)
                    for length, ratios in ratios_by_len.items()
                }

                # Transform the ratios for each length based on which types appear
                if has_coverage_unstable and not has_state_unstable:
                    for length in avg_ratios_by_len:
                        ratios = avg_ratios_by_len[length]
                        avg_ratios_by_len[length] = [
                            ratios[0]
                            + ratios[1],  # Combine both unstable and coverage unstable
                            0,
                            0,
                            ratios[2]
                            + ratios[3],  # Combine state unstable and both stable
                        ]
                elif has_state_unstable and not has_coverage_unstable:
                    for length in avg_ratios_by_len:
                        ratios = avg_ratios_by_len[length]
                        avg_ratios_by_len[length] = [
                            ratios[0]
                            + ratios[2],  # Combine both unstable and state unstable
                            0,
                            0,
                            ratios[1]
                            + ratios[3],  # Combine coverage unstable and both stable
                        ]

                # Calculate number of samples for each length
                samples_by_len = {
                    length: len(ratios) for length, ratios in ratios_by_len.items()
                }
                # Add overall count
                total_samples = sum(samples_by_len.values())
                samples_by_len[-1] = total_samples

                # Add overall average to the plot data
                input_lens = [-2] + input_lens[1:]
                avg_ratios_by_len[-2] = avg_ratios

                # Calculate bar widths based on number of samples, with fixed width for overall average
                max_samples = max(
                    samples_by_len[length] for length in input_lens[1:]
                )  # Exclude overall average
                min_width = 0.3  # Minimum bar width
                fixed_width = 1  # Width for overall average bar
                widths = [fixed_width] + [
                    min_width + (samples_by_len[length] / max_samples / 1.4)
                    for length in input_lens[1:]
                ]

                bottoms = np.zeros(len(input_lens))
                # Store transition points for horizontal lines
                weighted_avg_transitions = []

                # Use the same labels and colors as determined for the pie chart
                for i in range(4):  # For each ratio type
                    if labels[i]:  # Only plot if the label is not empty
                        values = [avg_ratios_by_len[length][i] for length in input_lens]
                        if any(v > 0 for v in values):
                            plt.bar(
                                input_lens,
                                values,
                                bottom=bottoms,
                                label=labels[i],
                                color=colors[i],
                                width=widths,
                            )
                            # Store the weighted average transition point
                            weighted_avg_transitions.append(bottoms[0] + values[0])
                        bottoms += values

                # Add horizontal lines at weighted average transitions
                x_min, x_max = plt.xlim()

                # Draw horizontal lines for transitions
                for i, y_val in enumerate(weighted_avg_transitions[:-1]):
                    plt.hlines(
                        y=y_val,
                        xmin=x_min,
                        xmax=x_max,
                        colors=colors[i],
                        linestyles="--",
                    )

                # Add the transition values to the y-axis ticks
                ax = plt.gca()
                ax.yaxis.set_major_formatter(plt.FormatStrFormatter("%.3f"))

                # Get default ticks and transition points
                default_ticks = list(plt.yticks()[0])  # Get current ticks
                transition_points = weighted_avg_transitions[
                    :-1
                ]  # Get transition points

                # Keep ticks that are far enough from transition points
                min_distance = 0.025  # Minimum distance between ticks
                kept_ticks = []
                for tick in default_ticks:
                    # Check if this tick is far enough from all transition points
                    if all(abs(tick - tp) > min_distance for tp in transition_points):
                        kept_ticks.append(tick)

                # Adjust transition points if they are too close together
                adjusted_transitions = transition_points.copy()
                for i in range(1, len(adjusted_transitions)):
                    if (
                        abs(adjusted_transitions[i] - adjusted_transitions[i - 1])
                        < min_distance
                    ):
                        # Move the current point up by min_distance
                        adjusted_transitions[i] = (
                            adjusted_transitions[i - 1] + min_distance
                        )

                # Combine kept ticks with adjusted transition points
                all_values = sorted(set(kept_ticks + adjusted_transitions))

                # Create tick labels with appropriate colors
                tick_labels = []
                for val in all_values:
                    # Check if this value is a transition point (using original transition points for color mapping)
                    if val in adjusted_transitions:
                        # Find which transition point it corresponds to
                        idx = adjusted_transitions.index(val)
                        tick_labels.append(
                            {
                                "label": f"{transition_points[idx]:.3f}",
                                "color": colors[idx],
                            }
                        )
                    else:
                        # Regular tick label in black
                        tick_labels.append({"label": f"{val:.3f}", "color": "black"})

                # Set the ticks and their properties
                plt.yticks(all_values, [t["label"] for t in tick_labels])
                # Color the tick labels
                ax.yaxis.set_tick_params(labelsize=10)
                for tick, tick_props in zip(ax.yaxis.get_ticklabels(), tick_labels):
                    tick.set_color(tick_props["color"])

                plt.xlabel(
                    "Input Length\n(Bar width indicates relative number of samples)",
                    labelpad=-20,
                )
                plt.ylabel("Ratio")
                plt.legend(framealpha=1.0)

                # Customize x-axis labels with sample counts
                x_labels = [f"Weighted\nAverage"] + [str(x) for x in input_lens[1:]]
                plt.xticks(input_lens, x_labels, rotation=90)

                # Adjust x-axis limits to reduce space between bars and edges
                plt.xlim(input_lens[0] - 0.8, input_lens[-1] + 0.8)
                plt.ylim(0, 1)

                plt.tight_layout()
                plt.savefig(bar_output_file)
                plt.close()
                print(f"Stacked bar chart saved to {bar_output_file}")
            else:
                print("No matching data found in the file")
    except FileNotFoundError:
        print(f"Error: File '{args.input_file}' not found")


if __name__ == "__main__":
    main()
