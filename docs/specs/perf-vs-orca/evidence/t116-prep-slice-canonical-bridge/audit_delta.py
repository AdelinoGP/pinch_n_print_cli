#!/usr/bin/env python3
"""Audit the ticket-116 candidate-vs-baseline G-code delta, section by section.

The candidate replaces PnP's flat-bridge enclosure discriminator with canonical
OrcaSlicer's rule, so the *expected* delta is confined to the flat-unsupported
bridge family and the roles that consume it. Ticket 116's stop rule says any
movement outside that family (perimeters, support geometry, tool paths outside
``bridge_areas``) is a parity bug, not noise.

This script splits both files into per-layer, per-``;TYPE:`` sections and
classifies every changed line:

  * ``extrusion_only`` — identical ``G0..G3`` motion (X/Y/Z/F tokens) with only
    the ``E`` token differing. This is a flow / role change on a segment whose
    path did not move (the per-vertex bridge flag feeding wall flow).
  * ``appendix``       — a ``;`` comment: the config appendix and the
    filament/time estimates. Includes the retired ``flat_bridge_closing_join``
    row.
  * ``progress_counter`` — an ``M73`` progress line, whose P/Q numbers follow
    the time estimate.
  * ``geometry``       — anything else: a moved toolpath. **Geometry lines in a
    structural section are the stop-rule failure.**

Usage:
    python audit_delta.py --baseline b.gcode --candidate c.gcode --cell <cell> --out row.json
"""

import argparse
import difflib
import hashlib
import json
import re
from collections import defaultdict

TYPE_RE = re.compile(r"^;TYPE:(.+)$")
Z_RE = re.compile(r"^;Z:([0-9.]+)$")
E_TOKEN_RE = re.compile(r"\sE-?[0-9.]+")

# Section types that must not move geometrically: perimeters, support geometry,
# and the first-layer / adhesion families.
STRUCTURAL_TYPES = {
    "Outer wall",
    "Inner wall",
    "Overhang wall",
    "Support",
    "Support interface",
    "Support transition",
    "Skirt",
    "Brim",
    "Raft",
}


def sections(path):
    """Map (z, type) -> list of lines in emission order."""
    out = defaultdict(list)
    z = "none"
    current = "startup"
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            line = raw.rstrip("\n")
            match = Z_RE.match(line)
            if match:
                z = match.group(1)
                current = "layer-change"
                continue
            match = TYPE_RE.match(line)
            if match:
                current = match.group(1)
                continue
            out[(z, current)].append(line)
    return out


def strip_extrusion(line):
    return E_TOKEN_RE.sub("", line)


def classify_pair(before, after):
    if strip_extrusion(before) == strip_extrusion(after):
        return "extrusion_only"
    if before.startswith(";") and after.startswith(";"):
        return "appendix"
    if before.startswith("M73 ") and after.startswith("M73 "):
        return "progress_counter"
    return "geometry"


def classify_single(line):
    if line.startswith(";"):
        return "appendix"
    if line.startswith("M73 "):
        return "progress_counter"
    return "geometry"


def sha256(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def audit(baseline_path, candidate_path, cell):
    base = sections(baseline_path)
    cand = sections(candidate_path)
    keys = set(base) | set(cand)

    per_type = defaultdict(lambda: defaultdict(int))
    appendix_samples = set()
    geometry_samples = []
    first_changed_layer = {}

    for key in keys:
        base_lines = base.get(key, [])
        cand_lines = cand.get(key, [])
        if base_lines == cand_lines:
            continue
        z, section = key
        matcher = difflib.SequenceMatcher(None, base_lines, cand_lines)
        for tag, i1, i2, j1, j2 in matcher.get_opcodes():
            if tag == "equal":
                continue
            before = base_lines[i1:i2]
            after = cand_lines[j1:j2]
            buckets = per_type[section]
            if tag == "replace" and len(before) == len(after):
                for b_line, a_line in zip(before, after):
                    kind = classify_pair(b_line, a_line)
                    buckets[kind] += 1
                    if kind == "appendix":
                        appendix_samples.add(b_line.strip())
                    if (
                        kind == "geometry"
                        and section in STRUCTURAL_TYPES
                        and len(geometry_samples) < 10
                    ):
                        geometry_samples.append(
                            {
                                "z": z,
                                "section": section,
                                "before": b_line,
                                "after": a_line,
                            }
                        )
            else:
                for b_line in before:
                    kind = classify_single(b_line)
                    buckets[kind] += 1
                    if kind == "appendix":
                        appendix_samples.add(b_line.strip())
                    if (
                        kind == "geometry"
                        and section in STRUCTURAL_TYPES
                        and len(geometry_samples) < 10
                    ):
                        geometry_samples.append(
                            {
                                "z": z,
                                "section": section,
                                "before": b_line,
                                "after": None,
                            }
                        )
                for a_line in after:
                    kind = classify_single(a_line)
                    buckets[kind] += 1
                    if kind == "appendix":
                        appendix_samples.add(a_line.strip())
                    if (
                        kind == "geometry"
                        and section in STRUCTURAL_TYPES
                        and len(geometry_samples) < 10
                    ):
                        geometry_samples.append(
                            {
                                "z": z,
                                "section": section,
                                "before": None,
                                "after": a_line,
                            }
                        )
        if z != "none" and (
            section not in first_changed_layer
            or float(z) < first_changed_layer[section]
        ):
            first_changed_layer[section] = float(z)

    # The fill/bridge families are *expected* to move toolpaths — that is the
    # classification's whole point — so only structural geometry counts as an
    # escape. Keep the structural samples for the failure message and total the
    # fill geometry for the record.
    fill_geometry_lines = sum(
        buckets.get("geometry", 0)
        for section, buckets in per_type.items()
        if section not in STRUCTURAL_TYPES
    )

    changed_types = {
        section: {"changed_lines": sum(buckets.values()), **dict(buckets)}
        for section, buckets in sorted(per_type.items())
    }

    structural = {k: v for k, v in changed_types.items() if k in STRUCTURAL_TYPES}
    structural_geometry_lines = sum(v.get("geometry", 0) for v in structural.values())

    return {
        "cell": cell,
        "baseline_sha256": sha256(baseline_path),
        "candidate_sha256": sha256(candidate_path),
        "layer_count": len({k[0] for k in keys if k[0] != "none"}),
        "changed_types": changed_types,
        "structural_changed_types": structural,
        "structural_geometry_lines": structural_geometry_lines,
        "structural_geometry_identical": structural_geometry_lines == 0,
        "appendix_changed_lines": sorted(appendix_samples),
        "structural_geometry_samples": geometry_samples,
        "fill_geometry_lines": fill_geometry_lines,
        "first_changed_z": {
            k: first_changed_layer[k] for k in sorted(first_changed_layer)
        },
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--cell", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()

    result = audit(args.baseline, args.candidate, args.cell)
    with open(args.out, "w", encoding="utf-8") as handle:
        json.dump(result, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(
        f"{args.cell}: changed_types={list(result['changed_types'])} "
        f"structural_geometry_lines={result['structural_geometry_lines']}"
    )


if __name__ == "__main__":
    main()
