#!/usr/bin/env python3
import re
from typing import Dict, List, Tuple


def extract_data(logfile: str) -> Tuple[Dict[int, List[List[int]]], int]:
    """
    Extract consistency ratio data from the log file.

    Args:
        logfile: Path to the log file to process

    Returns:
        Tuple containing:
        - Dictionary mapping input lengths to their data entries
        - Count of unfixed inputs
    """
    data_by_len = {}  # Dictionary to store data by input length
    pattern = r"consistency_ratios for input of len (\d+):\s+([\d,\s]+)"
    unfixed_count = 0

    with open(logfile) as f:
        for line in f:
            if match := re.search(pattern, line):
                input_len = int(match.group(1))
                values = [int(i) for i in match.group(2).split(",")]
                if input_len not in data_by_len:
                    data_by_len[input_len] = []
                data_by_len[input_len].append(values)
            elif "Input inconsistent" in line:
                unfixed_count += 1
            elif "tries, still unstable" in line:
                print(line[:-1])

    return data_by_len, unfixed_count


def calculate_global_ranges(
    data_by_len: Dict[int, List[List[int]]]
) -> Dict[str, Tuple[int, int]]:
    """
    Calculate global ranges for x-axes across all plots.

    Args:
        data_by_len: Dictionary mapping input lengths to their data entries

    Returns:
        Dictionary containing ranges for ratio, length, and filtered data
    """
    all_data = [item for sublist in data_by_len.values() for item in sublist]
    all_diffs = [x[0] - y for x in all_data for y in x]
    filtered_diffs = [x[0] - y for x in all_data if len(x) >= 2 for y in x]

    return {
        "ratio": (min(all_diffs), max(all_diffs)),
        "length": (min(len(x) for x in all_data), max(len(x) for x in all_data)),
        "filtered": (
            (min(filtered_diffs), max(filtered_diffs))
            if filtered_diffs
            else (min(all_diffs), max(all_diffs))
        ),
    }


def calculate_error_ratios(
    data_by_len: Dict[int, List[List[int]]]
) -> Tuple[List[int], List[float], List[float], Dict[int, Dict[str, List[float]]]]:
    """
    Calculate various error ratios from the data.

    Args:
        data_by_len: Dictionary mapping input lengths to their data entries

    Returns:
        Tuple containing:
        - List of input lengths
        - List of second-to-first ratios
        - List of sum-to-first ratios
        - Dictionary containing ratios by input length
    """
    lens = []
    second_to_first_ratios = []
    sum_to_first_ratios = []
    ratios_by_len = {}

    for input_len, len_data in sorted(data_by_len.items()):
        second_ratios_for_len = []
        sum_ratios_for_len = []
        for sublist in len_data:
            if len(sublist) >= 2:
                lens.append(input_len)
                second_ratio = sublist[1] / sublist[0]
                second_to_first_ratios.append(second_ratio)
                second_ratios_for_len.append(second_ratio)

                sum_ratio = sum(sublist[1:]) / sublist[0]
                sum_to_first_ratios.append(sum_ratio)
                sum_ratios_for_len.append(sum_ratio)

        if second_ratios_for_len:
            ratios_by_len[input_len] = {
                "second": second_ratios_for_len,
                "sum": sum_ratios_for_len,
            }

    return lens, second_to_first_ratios, sum_to_first_ratios, ratios_by_len
