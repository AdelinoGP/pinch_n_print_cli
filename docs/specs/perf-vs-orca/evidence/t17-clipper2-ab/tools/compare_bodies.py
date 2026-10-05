#!/usr/bin/env python
"""Per-base-name body comparison: monomorphizations kept distinct.

For each function base name (hash stripped), compare the SET of compiled bodies
between versions. A name whose bodies differ only by which monomorphizations
exist is not a semantic change in any called function.
"""

import re
import sys
from collections import defaultdict

HASH = re.compile(r"17h[0-9a-f]{16}E")
SKIP = (
    ".file",
    ".loc",
    ".cfi",
    ".p2align",
    ".section",
    ".globl",
    ".type",
    ".size",
    ".addrsig",
    ".seh",
    ".def",
    ".scl",
    ".endef",
    ".long",
    ".short",
    ".byte",
    ".quad",
    ".asciz",
    ".linkonce",
    ".weak",
    ".rva",
    ".xdata",
)


def norm(line):
    line = line.rstrip()
    if not line:
        return ""
    ls = line.lstrip()
    if ls.startswith(".") and any(k in ls for k in SKIP):
        return ""
    line = HASH.sub("::hHASHE", line)
    line = re.sub(r"\banon\.[0-9a-f]{32}\.\d+", "anon.HASH.N", line)
    line = re.sub(r"\bLBB\d+_\d+\b", "LBBX", line)
    line = re.sub(r"\bLtmp\d+\b", "LTMPX", line)
    line = re.sub(r"\.LJTI\d+_\d+", ".LJTIX", line)
    line = re.sub(r"\$LN\d+", "$LNX", line)
    line = re.sub(r"\.rva\s+\S+", ".rva X", line)
    return line


def parse_bodies(path):
    """base_name -> set(frozenset/tuple of instruction lines)."""
    out = defaultdict(list)
    cur = None
    buf = []
    for raw in open(path, encoding="utf-8", errors="replace"):
        s = raw.strip()
        if s.startswith("$") or s.startswith("anon.") or s.startswith("."):
            continue
        m = re.match(r"^([_A-Za-z][_A-Za-z0-9$.]*):\s*$", s)
        if m:
            if cur is not None and buf:
                out[cur].append(tuple(buf))
            cur = HASH.sub("::hHASHE", m.group(1))
            buf = []
            continue
        if cur is not None:
            n = norm(raw)
            if n:
                buf.append(n)
    if cur is not None and buf:
        out[cur].append(tuple(buf))
    return out


def main():
    a = parse_bodies(sys.argv[1])
    b = parse_bodies(sys.argv[2])
    names = sorted(set(a) | set(b))
    only_a = [n for n in names if n not in b]
    only_b = [n for n in names if n not in a]
    differing = []
    for n in names:
        if n in a and n in b and set(a[n]) != set(b[n]):
            differing.append(n)
    print(
        f"base names: 103={len(a)} 110={len(b)} common={len(names) - len(only_a) - len(only_b)}"
    )
    print(f"only-103={len(only_a)} only-110={len(only_b)} differing={len(differing)}")
    for n in only_a:
        print(f"  only-103: {n[:130]}  ({len(a[n])} bodies)")
    for n in only_b:
        print(f"  only-110: {n[:130]}  ({len(b[n])} bodies)")
    for n in differing:
        print(f"  DIFF: {n[:130]}")
        sa, sb = set(a[n]), set(b[n])
        print(
            f"    103={len(a[n])} bodies, 110={len(b[n])} bodies, shared={len(sa & sb)}, "
            f"a-only={len(sa - sb)}, b-only={len(sb - sa)}"
        )


if __name__ == "__main__":
    main()
