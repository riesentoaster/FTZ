#!/usr/bin/env python3

import argparse
import json
import shutil
import sys
import base64
from pathlib import Path
from multiprocessing import Pool, cpu_count


def extract_pcap(args: tuple[Path, Path, int]) -> None:
    metadata_file, output_dir, index = args
    try:
        with metadata_file.open() as f:
            data = json.load(f)
            pcap_base64 = next(
                (
                    entry[1]["pcap"]
                    for entry in data["metadata"]["map"].values()
                    if isinstance(entry, list)
                    and len(entry) > 1
                    and isinstance(entry[1], dict)
                    and entry[1].get("pcap")
                ),
                None,
            )
            if pcap_base64:
                output_path = output_dir / metadata_file.parent.relative_to(
                    metadata_file.parent.parent
                )
                output_path.mkdir(parents=True, exist_ok=True)
                (output_path / f"{index}.pcap").write_bytes(
                    base64.b64decode(pcap_base64)
                )
    except Exception as e:
        print(f"Error processing {metadata_file}: {e}")


def copy_files(args: tuple[Path, Path, Path, int]) -> None:
    metadata_file, input_dir, output_dir, index = args
    try:
        output_path = output_dir / metadata_file.parent.relative_to(input_dir)
        output_path.mkdir(parents=True, exist_ok=True)
        for file in metadata_file.parent.glob(f"{metadata_file.stem}*"):
            shutil.copy2(file, output_path / f"{index}{file.suffix}")
    except Exception as e:
        print(f"Error copying {metadata_file}: {e}")


def extract_packets(metadata_file: Path) -> tuple[Path, tuple[str, ...]]:
    try:
        with metadata_file.open() as f:
            data = json.load(f)
            packets = [
                p[1]
                for entry in data["metadata"]["map"].values()
                if isinstance(entry, list)
                and len(entry) > 1
                and isinstance(entry[1], dict)
                for p in entry[1].get("packets", [])
                if isinstance(p, list) and len(p) == 2
            ]
            return metadata_file, tuple(packets)
    except Exception as e:
        print(f"Error processing {metadata_file}: {e}")
        return metadata_file, tuple()


def main():
    parser = argparse.ArgumentParser("Extract files and PCAP data from metadata files")
    parser.add_argument("input", help="Input directory containing metadata files")
    parser.add_argument("output", help="Output directory for extracted files")
    parser.add_argument(
        "--force", action="store_true", help="Remove existing output directory"
    )
    parser.add_argument(
        "--pcap-only", action="store_true", help="Only extract PCAP files"
    )

    args = parser.parse_args()
    input_dir = Path(args.input).resolve()
    output_dir = Path(args.output).resolve()

    if not input_dir.is_dir():
        sys.exit(f"Input directory '{input_dir}' does not exist")
    if output_dir.is_dir():
        if args.force:
            shutil.rmtree(output_dir)
        else:
            sys.exit(f"Output directory '{output_dir}' already exists")

    output_dir.mkdir(parents=True, exist_ok=True)
    metadata_files = list(input_dir.rglob("*.metadata"))

    with Pool(processes=cpu_count()) as pool:
        # Deduplicate files based on packet content
        unique_files = []
        seen_packets = set()
        for metadata_file, packets in pool.map(extract_packets, metadata_files):
            if packets and packets not in seen_packets:
                unique_files.append(metadata_file)
                seen_packets.add(packets)

        print(
            f"Found {len(unique_files)} unique files out of {len(metadata_files)} total"
        )

        # Process files
        file_indices = [(f, output_dir, i) for i, f in enumerate(unique_files)]
        pool.map(extract_pcap, file_indices)

        if not args.pcap_only:
            pool.map(
                copy_files,
                [(f, input_dir, output_dir, i) for i, f in enumerate(unique_files)],
            )
            print(f"Processed {len(unique_files)} unique files to {output_dir}")
        else:
            print(f"Extracted {len(unique_files)} unique PCAP files to {output_dir}")


if __name__ == "__main__":
    main()
