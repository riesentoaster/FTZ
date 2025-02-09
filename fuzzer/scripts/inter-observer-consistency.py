import argparse, re, matplotlib.pyplot as plt, os
import numpy as np
from collections import defaultdict


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input_file", help="Input file to process")
    args = parser.parse_args()

    base_output_file = os.path.splitext(args.input_file)[0]
    pie_output_file = f"{base_output_file}-observer-ratios.svg"
    bar_output_file = f"{base_output_file}-observer-ratios-by-len.svg"

    # Dictionary to store ratios by input length
    ratios_by_len = defaultdict(list)

    try:
        with open(args.input_file) as f:
            for line in f:
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

                # Create pie chart
                plt.figure(figsize=(10, 8))
                plt.pie(
                    avg_ratios,
                    labels=[
                        "Both Unstable",
                        "Coverage Unstable/States Stable",
                        "Coverage Stable/States Unstable",
                        "Both Stable",
                    ],
                    colors=["#ff9999", "#66b3ff", "#99ff99", "#ffcc99"],
                    autopct="%1.5f%%",
                )
                plt.title(f"Observer Consistency Analysis")
                plt.axis("equal")
                plt.savefig(pie_output_file)
                plt.close()
                print(f"Pie chart saved to {pie_output_file}")

                # Create stacked bar chart
                plt.figure(figsize=(15, 8))
                input_lens = sorted(ratios_by_len.keys())
                avg_ratios_by_len = {
                    length: np.mean(ratios, axis=0)
                    for length, ratios in ratios_by_len.items()
                }

                # Calculate number of samples for each length
                samples_by_len = {
                    length: len(ratios) for length, ratios in ratios_by_len.items()
                }
                # Add overall count
                total_samples = sum(samples_by_len.values())
                samples_by_len[-1] = total_samples

                # Add overall average to the plot data
                input_lens = [-1] + input_lens  # Add -1 for overall average
                avg_ratios_by_len[-1] = avg_ratios  # Add overall average ratios

                # Calculate bar widths based on number of samples, with fixed width for overall average
                max_samples = max(
                    samples_by_len[length] for length in input_lens[1:]
                )  # Exclude overall average
                min_width = 0.1  # Minimum bar width
                fixed_width = 1  # Width for overall average bar
                widths = [fixed_width] + [
                    min_width + (samples_by_len[length] / max_samples / 1.3)
                    for length in input_lens[1:]
                ]

                bottoms = np.zeros(len(input_lens))
                colors = ["#ff9999", "#66b3ff", "#99ff99", "#ffcc99"]
                labels = [
                    "Both Unstable",
                    "Coverage Unstable/States Stable",
                    "Coverage Stable/States Unstable",
                    "Both Stable",
                ]

                for i in range(4):  # For each ratio type
                    values = [avg_ratios_by_len[length][i] for length in input_lens]
                    plt.bar(
                        input_lens,
                        values,
                        bottom=bottoms,
                        label=labels[i],
                        color=colors[i],
                        width=widths,
                    )
                    bottoms += values

                plt.xlabel(
                    "Input Length\n(Bar width indicates relative number of samples)"
                )
                plt.ylabel("Ratio")
                plt.title("Observer Consistency Analysis by Input Length")
                plt.legend()

                # Customize x-axis labels with sample counts
                x_labels = [f"Avg"] + [str(x) for x in input_lens[1:]]
                plt.xticks(input_lens, x_labels, rotation=0)

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
