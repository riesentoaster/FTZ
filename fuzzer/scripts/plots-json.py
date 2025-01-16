import argparse
import json
import math
from typing import Any, Dict, List

import matplotlib.axis as axis
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker


def is_numeric(value: Any) -> bool:
    """Check if a value is numeric (can be plotted)."""
    return isinstance(value, (int, float))


def extract_numeric_fields(
    data: Dict[str, Any], prefix: str = "", percent_fields: set = None
) -> Dict[str, float]:
    """Recursively extract all numeric fields from a JSON object."""
    result = {}
    for key, value in data.items():
        field_name = f"{prefix}{key}" if prefix else key
        if is_numeric(value):
            result[field_name] = float(value)
        elif isinstance(value, dict):
            if "Number" in value:
                # Use the Number value directly
                result[field_name] = float(value["Number"])
            elif "Percent" in value:
                result[field_name] = float(value["Percent"]) * 100
                if percent_fields is not None:
                    percent_fields.add(field_name)
            else:
                result.update(
                    extract_numeric_fields(value, f"{field_name}.", percent_fields)
                )
    return result


def plot(times: List[int], y: List[Any], ax: axis.Axis, ylabel: str, format_str=None):
    ax.plot(times, y)
    ax.set_xlabel("Time [s]")
    ax.set_ylabel(ylabel)
    if format_str:
        ax.yaxis.set_major_formatter(ticker.FormatStrFormatter(format_str))
    else:
        # If no format string is specified and it's not a percentage (which would have format_str set),
        # use integer formatting
        ax.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, p: "%d" % x))
    ax.grid()


def main():
    parser = argparse.ArgumentParser("Create plots from a JSON monitor output file.")
    parser.add_argument(
        "input",
        help="Path to the input file. Anything before the last dot will be used as the title and output filename.",
    )
    args = parser.parse_args()

    with open(args.input) as f:
        lines = f.readlines()
        # Extract all numeric fields from each line
        data_points = []
        fields = set()
        percent_fields = set()

        # Process all lines
        for line in lines:
            if not line.strip():
                continue
            json_data = json.loads(line)
            numeric_fields = extract_numeric_fields(
                json_data, percent_fields=percent_fields
            )
            data_points.append(numeric_fields)
            fields.update(numeric_fields.keys())

        # Sort fields for consistent ordering
        fields = sorted(fields)

        if "run_time" not in fields:
            raise ValueError("No run_time field found in data")

        times = [d["run_time"] for d in data_points]
        max_time = max(times)

        # Create configs for all fields except time
        configs = []
        for field in fields:
            if field == "run_time":  # Skip run_time field
                continue

            values = [d.get(field, 0) for d in data_points]

            # Determine if the field represents a percentage
            is_percent = field in percent_fields

            config = {
                "y": values,
                "ylabel": f"{field} [{' %' if is_percent else 'count'}]",
            }

            if is_percent:
                config["format_str"] = "%.3f"

            configs.append(config)
            min_time = min(times)
            if (max_time - min_time) < 3600:
                recent_indices = [i for i, t in enumerate(times) if t > max_time / 4]
            else:
                recent_indices = [i for i, t in enumerate(times) if max_time - t < 3600]
            if recent_indices:
                recent_times = [times[i] for i in recent_indices]
                recent_values = [values[i] for i in recent_indices]
                config = {
                    "y": recent_values,
                    "ylabel": f"Recent {field} [{' %' if is_percent else 'count'}]",
                    "times": recent_times,
                }
                if is_percent:
                    config["format_str"] = "%.3f"
                configs.append(config)

    plt_height = math.ceil(math.sqrt(len(configs)))
    plt_width = math.ceil(len(configs) / plt_height)

    fig, axes = plt.subplots(
        plt_height, plt_width, figsize=(7 * plt_width, 7 * plt_height + 1)
    )
    axes = axes.flatten()

    for config, ax in zip(configs, list(axes)):
        if "times" not in config:
            config["times"] = times
        plot(**config, ax=ax)

    base = ".".join(args.input.split(".")[:-1]) if "." in args.input else args.input
    fig.suptitle(base.split("/")[-1])
    fig.tight_layout()
    fig.savefig(f"{base}.png", dpi=100)


if __name__ == "__main__":
    main()
