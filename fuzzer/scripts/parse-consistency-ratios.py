#!/usr/bin/env python3
import argparse
from consistency.plotter import process_log_file


def main():
    parser = argparse.ArgumentParser(
        description="Parse and plot consistency ratios from log file."
    )
    parser.add_argument("logfile", help="Input log file to process")

    # Plot type selection
    plot_group = parser.add_mutually_exclusive_group()
    plot_group.add_argument(
        "--ratios-only",
        action="store_true",
        help="Generate only ratio plots (scatter and box plots)",
    )
    plot_group.add_argument(
        "--distributions-only",
        action="store_true",
        help="Generate only distribution plots",
    )

    # Other options
    parser.add_argument(
        "--print-counts", action="store_true", help="Print list length counts"
    )
    args = parser.parse_args()

    # Determine which plots to generate
    plot_ratios = not args.distributions_only
    plot_distributions = not args.ratios_only

    # Process the log file
    exit_code, ratio_file, dist_file = process_log_file(
        args.logfile,
        args.print_counts,
        plot_ratios=plot_ratios,
        plot_distributions=plot_distributions,
    )
    return exit_code


if __name__ == "__main__":
    exit(main())
