#!/usr/bin/env python3
import sys, re
from statistics import mean

patterns = {
    "load": re.compile(r"load average:\s*([\d.]+),\s*([\d.]+),\s*([\d.]+)"),
    "tasks": re.compile(
        r"Tasks:\s*(\d+)\s+total,\s*(\d+)\s+running,\s*(\d+)\s+sleeping,\s*(\d+)\s+stopped,\s*(\d+)\s+zombie"
    ),
    "cpu": re.compile(
        r"%Cpu\(s\):\s*([\d.]+)\s*us,\s*([\d.]+)\s*sy,\s*([\d.]+)\s*ni,\s*([\d.]+)\s*id,\s*([\d.]+)\s*wa,\s*([\d.]+)\s*hi,\s*([\d.]+)\s*si,\s*([\d.]+)\s*st"
    ),
    "mem": re.compile(
        r"MiB Mem\s*:\s*([\d.]+)\s*total,\s*([\d.]+)\s*free,\s*([\d.]+)\s*used,\s*([\d.]+)\s*buff/cache"
    ),
    "swap": re.compile(
        r"MiB Swap:\s*([\d.]+)\s*total,\s*([\d.]+)\s*free,\s*([\d.]+)\s*used\.\s*([\d.]+)\s*avail Mem"
    ),
}

data = {key: [] for key in patterns}

source = sys.stdin.read() if sys.stdin.isatty() == False else open(sys.argv[1]).read()
for line in source.splitlines():
    for key, pat in patterns.items():
        m = pat.search(line)
        if m:
            # Convert to float; tasks are integers but averaging as float is fine.
            data[key].append(tuple(float(x) for x in m.groups()))


def avg_tuple(lst):
    return tuple(mean(x) for x in zip(*lst)) if lst else ()


avgs = {k: avg_tuple(v) for k, v in data.items()}

print(
    f"top - load average: {avgs['load'][0]:.2f}, {avgs['load'][1]:.2f}, {avgs['load'][2]:.2f}"
)
print(
    f"Tasks: {avgs['tasks'][0]:.0f} total, {avgs['tasks'][1]:.0f} running, {avgs['tasks'][2]:.0f} sleeping, {avgs['tasks'][3]:.0f} stopped, {avgs['tasks'][4]:.0f} zombie"
)
print(
    f"%Cpu(s): {avgs['cpu'][0]:.1f} us, {avgs['cpu'][1]:.1f} sy, {avgs['cpu'][2]:.1f} ni, {avgs['cpu'][3]:.1f} id, {avgs['cpu'][4]:.1f} wa, {avgs['cpu'][5]:.1f} hi, {avgs['cpu'][6]:.1f} si, {avgs['cpu'][7]:.1f} st"
)
print(
    f"MiB Mem : {avgs['mem'][0]:.1f} total, {avgs['mem'][1]:.1f} free, {avgs['mem'][2]:.1f} used, {avgs['mem'][3]:.1f} buff/cache"
)
print(
    f"MiB Swap: {avgs['swap'][0]:.1f} total, {avgs['swap'][1]:.1f} free, {avgs['swap'][2]:.1f} used, {avgs['swap'][3]:.1f} avail Mem"
)
