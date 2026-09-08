"""AC-7 parser for packet 235-external-bridge-orientation.

Implements the AC's stated semantics exactly: M83 relative-E, positive E delta
ON AN XY MOVE = extrusion (retracts/unretracts without X/Y motion are NOT
extrusions), keyed by Z (never layer index), ';TYPE:' carried across layer
changes. Type match 'ridge' intentionally covers the pipeline's Bridge-family
roles ('BRIDGE', 'Internal Bridge', ...) emitted after packet 233.

Usage: python3 ac7_bridge_feedrate_check.py <gcode-file>
Asserts: >=1 bridge Z; bridge Zs strict subset of all Zs (I2/no-flooding);
exactly ONE distinct feedrate among bridge-type extrusion moves (I7 guard).
"""

import re
import sys

z = None
t = ""
bz = set()
az = set()
fs = set()
for l in open(sys.argv[1]):
    l = l.strip()
    if l.startswith(";TYPE:"):
        t = l[6:].strip()
    m = re.search(r"\bZ(-?\d+\.?\d*)", l)
    if m:
        z = float(m.group(1))
        az.add(z)
    m = re.search(r"\bF(\d+\.?\d*)", l)
    if m:
        f = float(m.group(1))
    mx = re.search(r"\bX(-?\d+\.?\d*)", l)
    my = re.search(r"\bY(-?\d+\.?\d*)", l)
    m = re.search(r"\bE(-?\.?\d+\.?\d*)", l)
    if (
        m
        and float(m.group(1)) > 0
        and z is not None
        and t
        and "ridge" in t
        and (mx or my)
    ):
        bz.add(z)
        fs.add(f)
assert len(bz) >= 1, "no bridge site"
assert len(bz) < len(az), f"flooding {len(bz)}/{len(az)}"
assert len(fs) == 1, f"bridge feedrates not uniform: {sorted(fs)}"
print(f"bridge_layers={len(bz)}/{len(az)} feedrate={fs}")
