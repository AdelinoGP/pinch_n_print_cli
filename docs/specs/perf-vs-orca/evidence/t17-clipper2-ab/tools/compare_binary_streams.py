#!/usr/bin/env python
"""Compare two stripped bench binaries at instruction level, ignoring addresses.

If the instruction streams are identical once numeric operands (addresses,
displacements) are normalized, the binaries differ only in link layout, and any
wall difference is alignment/branch-alias effects rather than code.

Usage: python compare_binary_streams.py <dumpA> <dumpB>
where <dump> comes from `llvm-objdump -d --no-show-raw-insn`.
"""

import re
import sys

NUM = re.compile(r"0x[0-9a-f]+|\b[0-9a-f]{4,}\b")
# rip-relative displacement / call targets appear as bare hex or 0x-hex
HEX = re.compile(r"\$?0x[0-9a-f]+")


def normalize(line):
    s = line.strip()
    if not s or s.endswith(":") and " " not in s.strip(":"):
        pass
    # drop the leading address column
    m = re.match(r"^([0-9a-f]+):\s+(.*)$", s)
    if m:
        s = m.group(2)
    # strip branch/call target comments
    s = s.split("#")[0].strip()
    # normalize all hex numbers and decimal immediates
    s = HEX.sub("0xN", s)
    s = NUM.sub("N", s)
    # normalize local branch labels like 140012345 <...>
    s = re.sub(r"N\s*<[^>]*>", "N <T>", s)
    s = re.sub(r"\s+", " ", s)
    return s


def stream(path):
    out = []
    for line in open(path, encoding="utf-8", errors="replace"):
        if not re.match(r"^\s*[0-9a-f]+:", line):
            continue
        n = normalize(line)
        if n:
            out.append(n)
    return out


def main():
    a = stream(sys.argv[1])
    b = stream(sys.argv[2])
    print(f"instructions: A={len(a)} B={len(b)}")
    if a == b:
        print("STREAMS IDENTICAL after address normalization")
        return
    import difflib

    d = list(difflib.unified_diff(a, b, lineterm="", n=0))
    print(f"diff lines: {len(d)}")
    shown = 0
    for line in d:
        if line.startswith(("+++", "---", "@@")):
            continue
        print(" ", line)
        shown += 1
        if shown >= 40:
            break


if __name__ == "__main__":
    main()
