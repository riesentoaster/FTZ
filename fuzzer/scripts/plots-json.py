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
    data: Dict[str, Any],
    prefix: str = "",
    percent_fields: set = None,
    float_format_fields: set = None,
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
            elif "Float" in value:
                # Use the Float value directly
                result[field_name] = float(value["Float"])
                if float_format_fields is not None:
                    float_format_fields.add(field_name)
            elif "Percent" in value:
                result[field_name] = float(value["Percent"]) * 100
                if percent_fields is not None:
                    percent_fields.add(field_name)
                if float_format_fields is not None:
                    float_format_fields.add(field_name)
            else:
                result.update(
                    extract_numeric_fields(
                        value,
                        f"{field_name}.",
                        percent_fields,
                        float_format_fields,
                    )
                )
    return result


def plot(
    times_list: List[List[int]],
    y_list: List[List[Any]],
    labels: List[str],
    ax: axis.Axis,
    ylabel: str,
    format_str=None,
):
    for times, y, label in zip(times_list, y_list, labels):
        ax.plot(times, y, label=label)
    ax.set_xlabel("Time [s]")
    ax.set_ylabel(ylabel)
    if format_str:
        ax.yaxis.set_major_formatter(ticker.FormatStrFormatter(format_str))
    else:
        # If no format string is specified and it's not a percentage (which would have format_str set),
        # use integer formatting
        ax.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, p: "%d" % x))
    ax.grid()
    ax.legend()


def process_file(filename: str) -> tuple[List[Dict[str, float]], set, set, set]:
    with open(filename) as f:
        lines = f.readlines()
        # Extract all numeric fields from each line
        data_points = []
        fields = set()
        percent_fields = set()
        float_format_fields = set()

        # Process all lines
        for line in lines:
            if not line.strip():
                continue
            json_data = json.loads(line)
            numeric_fields = extract_numeric_fields(
                json_data,
                percent_fields=percent_fields,
                float_format_fields=float_format_fields,
            )
            data_points.append(numeric_fields)
            fields.update(numeric_fields.keys())

    return data_points, fields, percent_fields, float_format_fields


def main():
    parser = argparse.ArgumentParser("Create plots from JSON monitor output files.")
    parser.add_argument(
        "inputs",
        nargs="+",
        help="Paths to the input files. The output will be named after the first file.",
    )
    parser.add_argument(
        "--include-recent",
        action="store_true",
        help="Include additional plots showing recent data (last quarter or last hour)",
    )
    args = parser.parse_args()

    # Process all input files
    all_data_points = []
    all_fields = set()
    all_percent_fields = set()
    all_float_format_fields = set()
    labels = []

    for input_file in args.inputs:
        data_points, fields, percent_fields, float_format_fields = process_file(
            input_file
        )
        all_data_points.append(data_points)
        all_fields.update(fields)
        all_percent_fields.update(percent_fields)
        all_float_format_fields.update(float_format_fields)
        # Use the filename without path and extension as the label
        label = input_file.split("/")[-1].rsplit(".", 1)[0]
        labels.append(label)

    # Sort fields for consistent ordering
    fields = sorted(all_fields)

    if "run_time" not in fields:
        raise ValueError("No run_time field found in data")

    # Create configs for all fields except time
    configs = []
    for field in fields:
        if field == "run_time":  # Skip run_time field
            continue

        times_list = []
        values_list = []
        for data_points in all_data_points:
            times = [d["run_time"] for d in data_points]
            values = [d.get(field, 0) for d in data_points]
            times_list.append(times)
            values_list.append(values)

        # Determine if the field represents a percentage
        is_percent = field in all_percent_fields
        use_float_format = field in all_float_format_fields

        config = {
            "times_list": times_list,
            "y_list": values_list,
            "labels": labels,
            "ylabel": f"{field} [{' %' if is_percent else 'count'}]",
        }

        if use_float_format:
            config["format_str"] = "%.3f"

        configs.append(config)

        # Only add recent plots if the flag is set
        if args.include_recent:
            recent_times_list = []
            recent_values_list = []

            for times, values in zip(times_list, values_list):
                max_time = max(times)
                min_time = min(times)
                if (max_time - min_time) < 3600:
                    recent_indices = [
                        i for i, t in enumerate(times) if t > max_time / 4
                    ]
                else:
                    recent_indices = [
                        i for i, t in enumerate(times) if max_time - t < 3600
                    ]
                if recent_indices:
                    recent_times = [times[i] for i in recent_indices]
                    recent_values = [values[i] for i in recent_indices]
                    recent_times_list.append(recent_times)
                    recent_values_list.append(recent_values)

            if recent_times_list:
                config = {
                    "times_list": recent_times_list,
                    "y_list": recent_values_list,
                    "labels": labels,
                    "ylabel": f"Recent {field} [{' %' if is_percent else 'count'}]",
                }
                if use_float_format:
                    config["format_str"] = "%.3f"
                configs.append(config)

    plt_height = math.ceil(math.sqrt(len(configs)))
    plt_width = math.ceil(len(configs) / plt_height)

    fig, axes = plt.subplots(
        plt_height, plt_width, figsize=(7 * plt_width, 7 * plt_height + 1)
    )
    axes = axes.flatten()

    for config, ax in zip(configs, list(axes)):
        plot(**config, ax=ax)

    base = (
        ".".join(args.inputs[0].split(".")[:-1])
        if "." in args.inputs[0]
        else args.inputs[0]
    )
    fig.suptitle(base.split("/")[-1])
    # fig.tight_layout()
    fig.savefig(f"{base}.svg", dpi=100)


if __name__ == "__main__":
    main()
