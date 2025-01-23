#!/usr/bin/env python3
from pathlib import Path
import base64, json, argparse
from multiprocessing import Pool


def process_file(f):
    (out_dir / f.stem).with_suffix(".pcap").write_bytes(
        base64.b64decode(json.loads(f.read_text())["pcap"])
    )


parser = argparse.ArgumentParser()
parser.add_argument("input_dir", type=Path, help="Input directory with .metadata files")
parser.add_argument("output_dir", type=Path, help="Output directory for .pcap files")
args = parser.parse_args()

in_dir, out_dir = args.input_dir, args.output_dir
out_dir.mkdir(exist_ok=True)

with Pool() as p:
    p.map(process_file, in_dir.glob("*.metadata"))
