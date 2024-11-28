#!/usr/bin/env python3

import argparse
import json
import shutil
import sys
import base64
from pathlib import Path
from typing import List


def process_metadata_file(file_path: Path) -> List[str]:
    packet_list = []
    try:
        with file_path.open("r", encoding="utf-8") as f:
            data = json.load(f)
        map_entries = data.get("metadata", {}).get("map", {})
        for entry in map_entries.values():
            if (
                isinstance(entry, list)
                and len(entry) >= 2
                and isinstance(entry[1], dict)
            ):
                if "packets" in entry[1].keys():
                    packets = entry[1].get("packets", [])
                    for packet in packets:
                        if isinstance(packet, list) and len(packet) == 2:
                            packet_str = packet[1]
                            if isinstance(packet_str, str):
                                packet_list.append(packet_str)
                            else:
                                print("packet is not string")
                        else:
                            print("packets list has weird entry")
                    pcap_base64 = entry[1].get("pcap", "")
                    pcap_data = base64.b64decode(pcap_base64)
                    pcap_file = file_path.with_suffix(".pcap")
                    pcap_file.write_bytes(pcap_data)
            else:
                print("Map entry has weird format")

    except Exception as e:
        print(f"Got exception when processing metadata file {file_path}: {e}")
    return packet_list


def remove_leading_dots(dir: Path) -> None:
    for path in dir.rglob("*"):
        if path.name.startswith("."):
            new_path = path.with_name(path.name.lstrip("."))
            if new_path.name and new_path != path:
                path.rename(new_path)


def copy_matching_files(
    metadata_file: Path, input_dir: Path, output_dir: Path, i: int
) -> None:
    stem = metadata_file.stem
    parent_dir = metadata_file.parent
    relative_parent = parent_dir.relative_to(input_dir)
    output_parent_dir = output_dir / relative_parent
    output_parent_dir.mkdir(parents=True, exist_ok=True)
    for file in parent_dir.glob(f"{stem}*"):
        shutil.copy2(file, f"{output_parent_dir}/{i}{file.suffix}")


def main():
    parser = argparse.ArgumentParser("Extract deduplicated files and pcap data.")
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument(
        "--force",
        action="store_true",
        help="Remove files from output.",
    )

    args = parser.parse_args()
    input_dir = Path(args.input).resolve()
    output_dir = Path(args.output).resolve()

    if not input_dir.is_dir():
        sys.exit(f"Input directory '{input_dir}' does not exist.")
    if output_dir.is_dir():
        if args.force:
            print(f"Cleaning old files from output dir {output_dir}")
        else:
            sys.exit(f"Output directory '{output_dir}' already exists.")

    dedup_metadata_files = []
    seen_packet_lists = set()

    metadata_files = list(input_dir.rglob("*.metadata"))

    for metadata_file in metadata_files:
        packet_list = process_metadata_file(metadata_file)
        packet_tuple = tuple(packet_list)
        if packet_tuple not in seen_packet_lists:
            dedup_metadata_files.append(metadata_file)
            seen_packet_lists.add(packet_tuple)

    print(
        f"From {len(metadata_files)} files, {len(dedup_metadata_files)} unique packet lists were identified."
    )

    for i, metadata_file in enumerate(dedup_metadata_files):
        copy_matching_files(metadata_file, input_dir, output_dir, i)


if __name__ == "__main__":
    main()
