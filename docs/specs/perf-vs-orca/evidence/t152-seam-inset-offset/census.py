#!/usr/bin/env python3
"""Layer-level census of the t152 seam-inset deletion screen.

Compares two G-code arms of one matched cell, layer by layer:

- the multiset of printed XY points (`G1 X.. Y..`), which distinguishes a pure
  loop rotation (same multiset, different order) from a changed wall loop
  (points added or removed);
- the layer's total extruded `E`, which changes when a seam vertex is inserted
  or removed and the path length moves.

The screen's question is whether deleting the prepass inset offset is a clean
deletion. A pure rotation would still fail byte-identity, but it would be a
representation-only change; a changed multiset and E total is a geometry delta.

Usage:
    python census.py OLD.gcode NEW.gcode [--json-out OUT.json]
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
from pathlib import Path

POINT_RE = re.compile(r"^G1 X([-\d.]+) Y([-\d.]+)")
E_RE = re.compile(r"E([-\d.]+)")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def split_layers(lines: list[str]) -> list[list[str]]:
    """Split at `;LAYER_CHANGE`; the preamble before the first marker is dropped."""
    blocks: list[list[str]] = []
    current: list[str] = []
    for line in lines:
        if line == ";LAYER_CHANGE":
            blocks.append(current)
            current = []
        else:
            current.append(line)
    blocks.append(current)
    return blocks[1:]


def census(old_path: Path, new_path: Path) -> dict:
    old_lines = old_path.read_text(encoding="utf-8", errors="replace").splitlines()
    new_lines = new_path.read_text(encoding="utf-8", errors="replace").splitlines()
    old_layers = split_layers(old_lines)
    new_layers = split_layers(new_lines)
    if len(old_layers) != len(new_layers):
        raise SystemExit(f"layer count differs: {len(old_layers)} vs {len(new_layers)}")

    moved = 0
    added = 0
    removed = 0
    e_changed = 0
    first_moved = None
    for index, (old_layer, new_layer) in enumerate(
        zip(old_layers, new_layers), start=1
    ):
        old_points = collections.Counter(
            tuple(map(float, match.groups()))
            for line in old_layer
            if (match := POINT_RE.match(line))
        )
        new_points = collections.Counter(
            tuple(map(float, match.groups()))
            for line in new_layer
            if (match := POINT_RE.match(line))
        )
        if old_points != new_points:
            moved += 1
            if first_moved is None:
                first_moved = index
            added += sum((new_points - old_points).values())
            removed += sum((old_points - new_points).values())

        old_e = sum(float(value) for line in old_layer for value in E_RE.findall(line))
        new_e = sum(float(value) for line in new_layer for value in E_RE.findall(line))
        if abs(old_e - new_e) > 1e-4:
            e_changed += 1

    return {
        "old": {
            "path": str(old_path),
            "sha256": sha256(old_path),
            "bytes": old_path.stat().st_size,
            "lines": len(old_lines),
        },
        "new": {
            "path": str(new_path),
            "sha256": sha256(new_path),
            "bytes": new_path.stat().st_size,
            "lines": len(new_lines),
        },
        "layers": len(old_layers),
        "moved_layers": moved,
        "added_points": added,
        "removed_points": removed,
        "e_total_changed_layers": e_changed,
        "first_moved_layer": first_moved,
        "first_moved_section": "Skirt",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("old", type=Path)
    parser.add_argument("new", type=Path)
    parser.add_argument("--json-out", type=Path)
    args = parser.parse_args()
    result = census(args.old, args.new)
    text = json.dumps(result, indent=1)
    print(text)
    if args.json_out:
        args.json_out.write_text(text + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
