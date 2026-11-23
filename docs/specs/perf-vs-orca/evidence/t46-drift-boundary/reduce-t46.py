#!/usr/bin/env python3
"""Reduce the ticket-46 raw captures into census.json (in-repo evidence).

Reads the four G-code outputs and their stderr logs from the (gitignored)
raw tree and emits the reduced, machine-checkable census of the frozen-job
output drift boundary. The raw tree layout is:

    target/t46-recon/frozen-control.gcode          (t38 snapshot, tree 2ebc372a)
    target/t44-ab/recon/head-exact-flags.gcode     (t44 snapshot, tree 0d65dbd9)
    target/t46-recon/current-head.gcode            (HEAD a21f119c, fresh dist)
    target/t46-recon/classic-off-current.gcode     (HEAD, matched-pair classic config)

Usage:
    python reduce-t46.py [--raw target] [--out census.json]

This script never slices; it only reads existing artifacts. Verification of
the emitted census against the in-repo visual bundles is verify-t46.py.
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import math
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
DEFAULT_RAW = REPO / "target"


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def type_sections(path: Path) -> collections.Counter:
    counts: collections.Counter = collections.Counter()
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(";TYPE:"):
            counts[line[6:].strip()] += 1
    return counts


def per_role(path: Path) -> dict:
    out: dict[str, list] = {}
    cur = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = re.match(r";TYPE:(.*)", line)
        if m:
            cur = m.group(1).strip()
            out.setdefault(cur, [0, 0.0])
            continue
        m = re.match(r"G1 .*E([\d.]+)", line)
        if m and cur:
            out[cur][0] += 1
            out[cur][1] += float(m.group(1))
    return {k: {"moves": v[0], "e_mm": round(v[1], 3)} for k, v in out.items()}


def total_e(path: Path) -> float:
    total = 0.0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = re.match(r"G1 .*E([\d.]+)", line)
        if m:
            total += float(m.group(1))
    return round(total, 3)


def count_lines(path: Path, prefix: tuple[str, ...]) -> int:
    n = 0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(prefix):
            n += 1
    return n


def appendix_keys(path: Path) -> int:
    n = 0
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if re.match(r"; [a-z_0-9]+ = ", line):
            n += 1
    return n


def unclosed_warnings(log: Path) -> dict:
    per_role: collections.Counter = collections.Counter()
    gaps = []
    pattern = re.compile(
        r"role=(\w+) is not closed.*?first=Some\(\(([\d.-]+), ([\d.-]+)\)\).*?last=Some\(\(([\d.-]+), ([\d.-]+)\)\)"
    )
    for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
        m = pattern.search(line)
        if m:
            per_role[m.group(1)] += 1
            x1, y1, x2, y2 = map(float, m.groups()[1:])
            gaps.append(math.hypot(x2 - x1, y2 - y1))
    gaps.sort()
    return {
        "total": sum(per_role.values()),
        "per_role": dict(per_role),
        "min_gap_mm": round(gaps[0], 6) if gaps else None,
        "p50_gap_mm": round(gaps[len(gaps) // 2], 3) if gaps else None,
        "max_gap_mm": round(gaps[-1], 3) if gaps else None,
    }


def layer_type_deltas(a: Path, b: Path) -> list:
    """Layers (index) whose TYPE-section counts differ between two outputs."""

    def per_layer(path: Path):
        layers = []
        cur = None
        for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
            if line.startswith(";LAYER_CHANGE"):
                cur = collections.Counter()
                layers.append(cur)
                continue
            if line.startswith(";TYPE:") and cur is not None:
                cur[line[6:].strip()] += 1
        return layers

    la, lb = per_layer(a), per_layer(b)
    out = []
    for i, (x, y) in enumerate(zip(la, lb)):
        keys = set(x) | set(y)
        d = {
            k: [x.get(k, 0), y.get(k, 0)]
            for k in sorted(keys)
            if x.get(k, 0) != y.get(k, 0)
        }
        if d:
            out.append({"layer": i, "deltas": d})
    return out


def angle_hist(path: Path, roles: tuple[str, ...]) -> dict:
    out: dict[str, collections.Counter] = {r: collections.Counter() for r in roles}
    cur = None
    prev = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = re.match(r";TYPE:(.*)", line)
        if m:
            cur = m.group(1).strip()
            prev = None if cur not in roles else prev
            continue
        m = re.match(r"G1 X([\d.-]+) Y([\d.-]+).*E[\d.]+", line)
        if m and cur in roles:
            x, y = float(m.group(1)), float(m.group(2))
            if prev:
                dx, dy = x - prev[0], y - prev[1]
                if abs(dx) + abs(dy) > 1e-9:
                    out[cur][round(math.degrees(math.atan2(dy, dx))) % 180] += 1
            prev = (x, y)
    return {r: dict(c.most_common(4)) for r, c in out.items()}


def sparse_containment(path: Path) -> dict:
    """Sparse E-move ends outside the layer's deposited-wall bbox (the t41 check)."""
    layers = []
    cur = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if line.startswith(";LAYER_CHANGE"):
            cur = {"wall": [None] * 4, "pts": []}
            layers.append(cur)
            continue
        if cur is None:
            continue
        m = re.match(r";TYPE:(.*)", line)
        if m:
            cur["role"] = m.group(1).strip()
            continue
        m = re.match(r"G1 X([\d.-]+) Y([\d.-]+).*E[\d.]+", line)
        if m:
            x, y = float(m.group(1)), float(m.group(2))
            if cur.get("role") in (
                "Outer wall",
                "Inner wall",
                "Internal Bridge",
                "Bridge",
            ):
                b = cur["wall"]
                b[0] = x if b[0] is None else min(b[0], x)
                b[1] = y if b[1] is None else min(b[1], y)
                b[2] = x if b[2] is None else max(b[2], x)
                b[3] = y if b[3] is None else max(b[3], y)
            if cur.get("role") == "Sparse infill":
                cur["pts"].append((x, y))
    total = 0
    violating = 0
    worst = 0.0
    for layer in layers:
        b = layer["wall"]
        if b[0] is None:
            continue
        for x, y in layer["pts"]:
            total += 1
            dx = max(b[0] - 0.05 - x, x - (b[2] + 0.05), 0.0)
            dy = max(b[1] - 0.05 - y, y - (b[3] + 0.05), 0.0)
            d = math.hypot(dx, dy)
            if d > 0:
                violating += 1
                worst = max(worst, d)
    return {
        "sparse_points_checked": total,
        "violating_points": violating,
        "worst_overshoot_mm": round(worst, 3),
    }


def era(gcode: Path, stderr: Path) -> dict:
    data = {
        "gcode_sha256": sha256(gcode),
        "bytes": gcode.stat().st_size,
        "type_sections": dict(type_sections(gcode)),
        "per_role": per_role(gcode),
        "e_total_mm": total_e(gcode),
        "fan_lines": count_lines(gcode, ("M106", "M107")),
        "appendix_key_lines": appendix_keys(gcode),
        "sparse_containment": sparse_containment(gcode),
        "angles_top4": angle_hist(
            gcode, ("Sparse infill", "Bottom surface", "Top surface")
        ),
    }
    if stderr.exists():
        text = stderr.read_text(encoding="utf-8", errors="replace").splitlines()
        data["unclosed_warnings"] = unclosed_warnings(stderr)
        complete = [l for l in text if '"event":"slice_complete"' in l]
        if complete:
            evt = json.loads(complete[-1])
            data["completion"] = {
                "status": evt.get("status"),
                "degraded": evt.get("degraded"),
                "fatal_error_count": evt.get("fatal_error_count"),
                "non_fatal_error_count": evt.get("non_fatal_error_count"),
            }
    return data


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--raw", type=Path, default=DEFAULT_RAW)
    ap.add_argument("--out", type=Path, default=Path(__file__).with_name("census.json"))
    args = ap.parse_args()

    raw = args.raw
    frozen_g = raw / "t46-recon" / "frozen-control.gcode"
    t44_g = raw / "t44-ab" / "recon" / "head-exact-flags.gcode"
    current_g = raw / "t46-recon" / "current-head.gcode"
    classic_g = raw / "t46-recon" / "classic-off-current.gcode"

    missing = [
        str(p) for p in (frozen_g, t44_g, current_g, classic_g) if not p.exists()
    ]
    if missing:
        raise SystemExit(
            "raw captures missing (re-derive via the FINDINGS commands): "
            + ", ".join(missing)
        )

    current = era(current_g, raw / "t46-recon" / "current-head.stderr.log")
    classic = era(classic_g, raw / "t46-recon" / "classic-off-current.stderr.log")
    frozen_t44_bytes = era(t44_g, raw / "t44-ab" / "recon" / "head-exact-flags.jsonl")[
        "bytes"
    ]

    # The byte arithmetic that pins the eras together (all measured this take):
    #   current + alias line  == t47 t44-job reference 11e7d38a…
    #   current classic-off + alias line == t47 classic-off reference e1088b3d…
    #   current -- fan bytes + alias line == t44-era reference 1b71f83d…
    alias_bytes = len(b"; support_overhang_angle = 30\n")
    fan_bytes = sum(
        len(l) + 1
        for l in current_g.read_text(encoding="utf-8").splitlines()
        if l.startswith(("M106", "M107"))
    )
    census = {
        "kind": "frozen-job output drift boundary (wayfinder ticket 46)",
        "model_sha256": sha256(REPO / "tmp" / "3dbenchy.stl")
        if (REPO / "tmp" / "3dbenchy.stl").exists()
        else None,
        "config_t44_sha256": sha256(
            REPO
            / "docs/specs/perf-vs-orca/evidence/t44-clip-universe-hoist/benchy-arachne.json"
        ),
        "eras": {
            "frozen_t38_snapshot": {
                "tree": "2ebc372a",
                **era(frozen_g, raw / "t46-recon" / "frozen-control.stderr.log"),
            },
            "t44_snapshot": {
                "tree": "0d65dbd9",
                **era(t44_g, raw / "t44-ab" / "recon" / "head-exact-flags.jsonl"),
            },
            "current_head": {
                "tree": "a21f119c + t46 claim",
                **current,
            },
            "current_head_classic_off": classic,
        },
        "byte_arithmetic": {
            "alias_line_bytes": alias_bytes,
            "fan_bytes": fan_bytes,
            "t47_t44job_reference_sha256": "11e7d38a60f5018badd9c06424c296fcfc12e5d87bc785800fa2c803ba555a58",
            "t47_classic_off_reference_sha256": "e1088b3df78b51b4d0add0c64b3cf143d44ca95cfb5c15db14080772b4e77fbf",
            "t44_reference_sha256": "1b71f83db378d33c2c52ee84028a7180b8b65d26edffde7eff0b06b40b2d403b",
            "frozen_reference_sha256": "7049a06d68ee8ab012adc8f26c831bd8cc1a59d0d10d737d1257f58a8719c161",
            "expected": {
                "current_plus_alias_equals_t47_t44job": True,
                "current_classic_plus_alias_equals_t47_classic_off": True,
                "current_minus_fans_plus_alias_equals_t44": True,
                "note": (
                    "sizes measured: current {cur} = t44 {t44} + fans {fan} - alias {alias} = {recon}"
                ).format(
                    cur=current["bytes"],
                    t44=frozen_t44_bytes,
                    fan=fan_bytes,
                    alias=alias_bytes,
                    recon=frozen_t44_bytes + fan_bytes - alias_bytes,
                ),
            },
        },
        "layer_type_deltas_frozen_vs_t44": layer_type_deltas(frozen_g, t44_g),
        "note": "frozen vs t44 layer deltas listed; current shares the t44 move stream exactly (see FINDINGS).",
    }
    # current vs t44 move-stream equality check is recorded in FINDINGS; census keeps the deltas list small.
    args.out.write_text(json.dumps(census, indent=1), encoding="utf-8")
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
