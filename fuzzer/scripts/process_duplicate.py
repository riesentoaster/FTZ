#!/usr/bin/env python3
import json, base64, sys
from pathlib import Path
from multiprocessing import Pool


def process(f):
    with open(f) as j, open(f.with_suffix(".pcap"), "wb") as p:
        p.write(base64.b64decode(json.load(j)["pcap"]))


with Pool() as p:
    p.map(process, Path(sys.argv[1]).glob("*.metadata"))
