#!/usr/bin/env python3
"""Bridge-site layer auditor for the packet-234 acceptance commands.

Parses a Pinch'n'Print G-code file with M83 relative-E semantics: a line is an
extrusion move when its E value is positive, `;TYPE:` markers are carried
across layer changes, and layers are keyed by Z (never layer index). A layer
counts as a bridge site when a "ridge"-containing `;TYPE:` (Bridge or
Internal Bridge) is current during at least one positive-E move.

Replaces the original inline `python3 -c "..."` AC commands in
`docs/spec_packets/234-bridge-false-site-gating/packet.spec.md` (their
embedded `\n` sequences were not runnable as written).

Usage:
  python3 resources/check_bridge_sites.py [--first-layers-zero N] <gcode>

Invariants asserted (AC-3 / AC-5):
  - at least one layer carries bridge-type extrusion (site exists);
  - bridge-type layers are a strict subset of all layers (no flooding);
  - with `--first-layers-zero N`, the N lowest layers carry zero
    bridge-type extrusion (solid-underneath demotion).

Prints `bridge_layers=N/M z=[...]` on success; exits non-zero on failure.
"""

import argparse
import re
import sys


def main(argv):
    parser = argparse.ArgumentParser(
        prog="check_bridge_sites",
        description=__doc__.splitlines()[0],
    )
    parser.add_argument(
        "--first-layers-zero",
        type=int,
        default=0,
        metavar="N",
        help="assert the N lowest layers carry zero bridge-type extrusion",
    )
    parser.add_argument("gcode", help="path to the generated G-code file")
    args = parser.parse_args(argv)

    z = None
    current_type = ""
    bridge_layers = set()
    all_layers = set()

    with open(args.gcode, encoding="utf-8") as handle:
        for raw in handle:
            line = raw.strip()
            if line.startswith(";TYPE:"):
                current_type = line[6:].strip()
            z_match = re.search(r"\bZ(-?\d+\.?\d*)", line)
            if z_match:
                z = float(z_match.group(1))
                all_layers.add(z)
            e_match = re.search(r"\bE(-?\d+\.?\d*)", line)
            if (
                e_match
                and float(e_match.group(1)) > 0
                and z is not None
                and current_type
                and "ridge" in current_type
            ):
                bridge_layers.add(z)

    sorted_layers = sorted(all_layers)
    assert len(bridge_layers) >= 1, "no bridge site"
    assert len(bridge_layers) < len(all_layers), (
        f"flooding {len(bridge_layers)}/{len(all_layers)}"
    )
    for i in range(min(args.first_layers_zero, len(sorted_layers))):
        assert sorted_layers[i] not in bridge_layers, (
            f"layer {sorted_layers[i]} carries bridge (solid lower layer)"
        )

    print(
        f"bridge_layers={len(bridge_layers)}/{len(all_layers)} "
        f"z={sorted(bridge_layers)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
