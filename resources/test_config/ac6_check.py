"""AC-6 invariant I7 checker (packet 233-internal-bridge-over-infill).

Asserts that every extruding move in an ';TYPE:Internal Bridge' section is
emitted at F2250 (internal_bridge_speed default 37.5 mm/s x 60) and that both
reslice outputs (--config varying only sparse_infill_speed) carry identical bridge
move feedrate lists. Non-vacuous: fails when zero bridge extrusions are found.

Usage: python3 resources/test_config/ac6_check.py <a.gcode> <b.gcode>
"""

import re
import sys


def bridge_feedrates(path):
    typ = None
    current_f = None
    out = []
    with open(path, errors="replace") as fh:
        for line in fh:
            if line.startswith(";TYPE:"):
                typ = line.strip()[len(";TYPE:") :]
                continue
            if not line.startswith("G1"):
                continue
            mf = re.search(r"F([\d.]+)", line)
            if mf:
                current_f = float(mf.group(1))
            # Extrusion = positive E delta on an XY move (packet gotcha);
            # E-only retract/re-prime lines are excluded.
            me = re.search(r" E([.\d]+)", line)
            if me and typ == "Internal Bridge" and "X" in line and "Y" in line:
                e = float(me.group(1))
                if e > 0.0:
                    out.append(current_f)
    return out


def main():
    a_path, b_path = sys.argv[1], sys.argv[2]
    a, b = bridge_feedrates(a_path), bridge_feedrates(b_path)
    assert len(a) > 0, (
        f"no Internal Bridge extruding moves found in {a_path} (vacuous check rejected)"
    )
    assert len(b) > 0, (
        f"no Internal Bridge extruding moves found in {b_path} (vacuous check rejected)"
    )
    assert a == b, f"feedrate lists differ across reslices ({len(a)} vs {len(b)} moves)"
    bad = sorted({f for f in a if f != 2250.0})
    assert not bad, f"I7 violated: non-2250 feedrates present: {bad[:5]}"
    print(
        f"AC-6 PASS: {len(a)} Internal Bridge extrusions identical across reslices, all F2250"
    )


if __name__ == "__main__":
    main()
