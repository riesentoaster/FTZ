#!/usr/bin/env python3

import sys
import argparse
from pathlib import Path
import subprocess
from collections import Counter


def get_addresses(sanitizer_cov_path, unstable_coverage_path):
    # First read sanitizer_cov.txt to build offset->addr mapping
    offset_to_addr = {}
    with open(sanitizer_cov_path) as f:
        for line in f:
            splited = line.strip().split(": ")
            if len(splited) == 2:
                offset, addr = splited
                offset_to_addr[offset] = addr

    # Read and count offsets in unstable-coverage.txt
    not_found_counts = Counter()
    addr_counts = Counter()

    with open(unstable_coverage_path) as f:
        for line in f:
            offset = line.strip()
            if offset:
                if offset in offset_to_addr:
                    addr_counts[offset_to_addr[offset]] += 1
                else:
                    not_found_counts[offset] += 1

    # Print statistics about not found offsets
    print(f"\nNot Found Offset Statistics:")
    not_found_hits = sum(not_found_counts.values())
    found_hits = sum(addr_counts.values())
    print(f"Total hits in {unstable_coverage_path}: {found_hits + not_found_hits}")
    print(f"Found hits: {found_hits}")
    print(f"Not found hits: {not_found_hits}")
    print(f"Unique not found offsets: {len(not_found_counts)}")
    print("\nNot found offset frequencies:")
    for offset, count in sorted(
        not_found_counts.items(), key=lambda x: x[1], reverse=True
    ):
        print(f"{count:5d} hits - offset {offset}")

    # Return addresses sorted by count (descending)
    return sorted(
        ((addr, count) for addr, count in addr_counts.items()),
        key=lambda x: x[1],
        reverse=True,
    )


def main():
    parser = argparse.ArgumentParser(description="Process stability coverage data.")
    parser.add_argument("executable", help="Path to the executable (e.g., zephyr.exe)")
    parser.add_argument("sanitizer_cov", help="Path to sanitizer_cov_unique.txt file")
    parser.add_argument("unstable_coverage", help="Path to unstable-coverage.txt file")
    args = parser.parse_args()

    exe = Path(args.executable)
    if not exe.exists():
        print(f"Error: {exe} not found")
        sys.exit(1)

    addr_counts = get_addresses(args.sanitizer_cov, args.unstable_coverage)
    if not addr_counts:
        print("No matching addresses found")
        sys.exit(0)

    print(f"\nFound {len(addr_counts)} matching addresses")

    # Run addr2line
    addrs = [addr for addr, _ in addr_counts]
    cmd = ["addr2line", "-e", str(exe), "-f", "-C"] + addrs
    try:
        out = subprocess.run(cmd, capture_output=True, text=True, check=True).stdout
    except subprocess.CalledProcessError as e:
        print(f"addr2line failed: {e.stderr}")
        sys.exit(1)

    # Print results in single line format, sorted by count
    print("\nAddress resolution results:")
    for (addr, count), func, loc in zip(
        addr_counts, out.strip().split("\n")[::2], out.strip().split("\n")[1::2]
    ):
        print(f"{count:5d} hits - {addr}: {func} at {loc}".replace("\n", ""))


if __name__ == "__main__":
    main()
