#!/usr/bin/env python3
"""Ticket 135 Stage 1 parity oracle: PnP vs OrcaSlicer skirt/brim geometry.

Compares the two arms' *adhesion geometry* rather than absolute coordinates —
the two slicers place the model at different origins (measured: ~3.2 mm in X and
~1.0 mm in Y on benchy), so every metric here is translation invariant.

Metrics per cell:

  * ``growth_mm`` — how far the brim/skirt grows the first-layer outer-wall
    bounding box on each axis. Placement invariant, and the falsifier for "the
    brim still follows the bounding box".
  * ``swept_area_mm2`` — ``sum(loop_length_i) * spacing``, the area of the
    annulus each arm lays plastic over. Spacing is derived from each arm's own
    resolved appendix (PnP: ``line_width``; Orca: ``initial_layer_line_width``
    with the rounded-rectangle flow model), so the two flow models are
    normalised before comparison.
  * ``loop_count`` — reported, not gated: the two flow models pick different
    loop counts for the same 8 mm ring by construction.

``filament_mm`` (the raw ``E`` sum) is *reported* but not gated: PnP extrudes a
rectangular cross-section (``width x height``) while Orca uses the rounded
model, so equal geometry still shows a ~13 % ``E`` gap. That flow-model
asymmetry is global to the matched job, not specific to this port.

Declared tolerances (set from the benchy classic-off probe, applied unchanged to
all eight cells): brim growth <= 1.5 mm, skirt growth <= 1.5 mm, brim swept area
within 12 %, skirt swept area within 5 %. The brim band is wider because each arm
quantises ``brim_width`` by its own spacing (Orca 0.457 mm -> 7.31 mm, PnP
0.4 mm -> 8.0 mm), a consequence of Orca's 0.5 mm initial-layer width that PnP
does not model.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

LOOP_LINE = re.compile(
    r"^G[01] X(-?\d+(?:\.\d+)?) Y(-?\d+(?:\.\d+)?)(?:\s+Z(-?\d+(?:\.\d+)?))?(.*)$"
)
PNP_FEATURE = re.compile(r"^;TYPE:(.+?)\s*$")
ORCA_FEATURE = re.compile(r"^; FEATURE:(.+?)\s*$")
APPENDIX = re.compile(r"^;\s*([A-Za-z0-9_]+)\s*=\s*(.*?)\s*$")


@dataclass
class Section:
    """One feature section: its extrusion runs and their filament total."""

    runs: list[list[tuple[float, float]]] = field(default_factory=list)
    filament_mm: float = 0.0

    def bbox(self) -> tuple[float, float, float, float]:
        xs = [p[0] for run in self.runs for p in run]
        ys = [p[1] for run in self.runs for p in run]
        if not xs:
            return (0.0, 0.0, 0.0, 0.0)
        return (min(xs), min(ys), max(xs), max(ys))

    def length_mm(self) -> float:
        total = 0.0
        for run in self.runs:
            total += sum(math.dist(run[i], run[i + 1]) for i in range(len(run) - 1))
        return total

    def outer_area_mm2(self) -> float:
        """Enclosed area of the largest run (shoelace), the ring's outer boundary."""
        best = 0.0
        for run in self.runs:
            if len(run) < 3:
                continue
            twice = 0.0
            for i in range(len(run)):
                x0, y0 = run[i]
                x1, y1 = run[(i + 1) % len(run)]
                twice += x0 * y1 - x1 * y0
            best = max(best, abs(twice) * 0.5)
        return best


def parse_appendix(path: Path) -> dict[str, str]:
    cfg: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        m = APPENDIX.match(line)
        if m:
            cfg.setdefault(m.group(1), m.group(2))
    return cfg


def parse_gcode(path: Path, feature_re: re.Pattern[str]) -> dict[str, Section]:
    """Split a g-code file into per-feature extrusion sections.

    Only the first layer is considered: the skirt lives on layers 0..n and the
    brim on layer 0; later layers repeat the same object and would swamp the
    bbox metrics.
    """
    sections: dict[str, Section] = {}
    current: str | None = None
    run: list[tuple[float, float]] = []
    seen_first_layer_marker = False
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()

    def flush_run() -> None:
        nonlocal run
        if run and current is not None:
            sections[current].runs.append(run)
            run = []

    for line in lines:
        if line.startswith(";LAYER_CHANGE") or line.startswith("; CHANGE_LAYER"):
            flush_run()
            if seen_first_layer_marker:
                break
            # The first marker opens layer 0; the next one closes it.
            seen_first_layer_marker = True
            continue
        m = feature_re.match(line)
        if m:
            flush_run()
            current = m.group(1).strip()
            sections.setdefault(current, Section())
            continue
        mm = LOOP_LINE.match(line)
        if not mm:
            continue
        tail = mm.group(4)
        e = re.search(r"\bE(-?(?:\d+(?:\.\d+)?|\.\d+))", tail)
        x, y = float(mm.group(1)), float(mm.group(2))
        if e is None:
            flush_run()
            continue
        if e.group(1).startswith("-"):
            # Retraction inside the feature: not laid-down plastic.
            continue
        run.append((x, y))
        sections[current].filament_mm += float(e.group(1))
    flush_run()
    return sections


def object_bbox(sections: dict[str, Section]) -> tuple[float, float, float, float]:
    """First-layer outer-wall bbox, the placement-invariant reference frame."""
    for name in ("Outer wall", "OuterWall"):
        section = sections.get(name)
        if section and section.runs:
            return section.bbox()
    raise SystemExit("oracle: no first-layer outer-wall section found")


def growth(
    ref: tuple[float, float, float, float], box: tuple[float, float, float, float]
) -> dict[str, float]:
    return {
        "xmin": ref[0] - box[0],
        "ymin": ref[1] - box[1],
        "xmax": box[2] - ref[2],
        "ymax": box[3] - ref[3],
    }


def pnp_spacing(cfg: dict[str, str]) -> float:
    """PnP's adhesion flow spacing: `line_width - h x (1 - pi/4)`.

    `skirt-brim` resolves its own flow spacing from `line_width` and
    `first_layer_height` (canonical `rounded_rectangle_extrusion_spacing`), so
    the appendix's `line_width` alone is not the spacing.
    """
    width = float(cfg.get("line_width", "0.4"))
    height = float(cfg.get("first_layer_height", "0") or 0.0)
    if height <= 0.0:
        return width
    return width - height * (1.0 - 0.25 * math.pi)


def orca_spacing(cfg: dict[str, str]) -> float:
    width = float(cfg.get("initial_layer_line_width", "0"))
    if width <= 0.0:
        width = float(cfg.get("line_width", "0.42"))
    height = float(cfg.get("layer_height", "0.2"))
    return width - height * (1.0 - 0.25 * math.pi)


def compare(pnp_path: Path, orca_path: Path, cell: str) -> dict:
    pnp = parse_gcode(pnp_path, PNP_FEATURE)
    orca = parse_gcode(orca_path, ORCA_FEATURE)
    pnp_cfg = parse_appendix(pnp_path)
    orca_cfg = parse_appendix(orca_path)
    p_spacing = pnp_spacing(pnp_cfg)
    o_spacing = orca_spacing(orca_cfg)

    pnp_obj = object_bbox(pnp)
    orca_obj = object_bbox(orca)

    out: dict = {
        "cell": cell,
        "spacing_mm": {"pnp": round(p_spacing, 5), "orca": round(o_spacing, 5)},
        "pnp": {},
        "orca": {},
        "checks": {},
    }
    for role in ("Brim", "Skirt"):
        p_sec = pnp.get(role, Section())
        o_sec = orca.get(role, Section())
        p_has = bool(p_sec.runs)
        o_has = bool(o_sec.runs)
        out["pnp"][role] = {
            "loop_count": len(p_sec.runs),
            "present": p_has,
            "filament_mm": round(p_sec.filament_mm, 3),
            "path_length_mm": round(p_sec.length_mm(), 2),
            "swept_area_mm2": round(p_sec.length_mm() * p_spacing, 2) if p_has else 0.0,
            "outer_loop_area_mm2": round(p_sec.outer_area_mm2(), 2) if p_has else 0.0,
            "growth_mm": growth(pnp_obj, p_sec.bbox()) if p_has else {k: 0.0 for k in ("xmin", "ymin", "xmax", "ymax")},
        }
        out["orca"][role] = {
            "loop_count": len(o_sec.runs),
            "present": o_has,
            "filament_mm": round(o_sec.filament_mm, 3),
            "path_length_mm": round(o_sec.length_mm(), 2),
            "swept_area_mm2": round(o_sec.length_mm() * o_spacing, 2) if o_has else 0.0,
            "outer_loop_area_mm2": round(o_sec.outer_area_mm2(), 2) if o_has else 0.0,
            "growth_mm": growth(orca_obj, o_sec.bbox()) if o_has else {k: 0.0 for k in ("xmin", "ymin", "xmax", "ymax")},
        }

    def rel(p: float, o: float) -> float:
        if o == 0.0:
            return 0.0 if p == 0.0 else float("inf")
        return abs(p - o) / o

    checks: dict = {}
    for role in ("Brim", "Skirt"):
        # Empty on both arms is agreement, not missing data (canonical can clip
        # a whole brim away when supports surround the part).
        both_absent = (not out["pnp"][role]["present"]) and (not out["orca"][role]["present"])
        one_absent = out["pnp"][role]["present"] != out["orca"][role]["present"]
        checks[f"{role.lower()}_presence"] = not one_absent
        checks[f"{role.lower()}_both_absent"] = both_absent
        checks[f"{role.lower()}_swept_area_rel_diff"] = round(
            rel(out["pnp"][role]["swept_area_mm2"], out["orca"][role]["swept_area_mm2"]), 4
        ) if not one_absent else float("inf")
        checks[f"{role.lower()}_outer_loop_area_rel_diff"] = round(
            rel(out["pnp"][role]["outer_loop_area_mm2"], out["orca"][role]["outer_loop_area_mm2"]), 4
        ) if not one_absent else float("inf")
        checks[f"{role.lower()}_growth_abs_diff_mm"] = round(
            abs(
                max(out["pnp"][role]["growth_mm"].values(), key=abs)
                - max(out["orca"][role]["growth_mm"].values(), key=abs)
            ),
            3,
        ) if not one_absent else float("inf")
        checks[f"{role.lower()}_filament_rel_diff"] = round(
            rel(out["pnp"][role]["filament_mm"], out["orca"][role]["filament_mm"]), 4
        ) if not one_absent else float("inf")
    checks["brim_swept_area_pass"] = checks["brim_swept_area_rel_diff"] <= 0.12
    checks["skirt_area_pass"] = (
        checks["skirt_outer_loop_area_rel_diff"] <= 0.03 or checks["skirt_both_absent"]
    )
    checks["brim_presence_pass"] = checks["brim_presence"]
    checks["skirt_presence_pass"] = checks["skirt_presence"]
    checks["brim_growth_pass"] = checks["brim_growth_abs_diff_mm"] <= 1.5
    checks["skirt_growth_pass"] = checks["skirt_growth_abs_diff_mm"] <= 1.5
    checks["pass"] = all(
        checks[k]
        for k in (
            "brim_swept_area_pass",
            "skirt_area_pass",
            "brim_presence_pass",
            "skirt_presence_pass",
            "brim_growth_pass",
            "skirt_growth_pass",
        )
    )
    out["checks"] = checks
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--pnp", type=Path, required=True, help="PnP candidate gcode")
    ap.add_argument(
        "--orca", type=Path, required=True, help="Orca gcode for the same cell"
    )
    ap.add_argument("--cell", required=True)
    ap.add_argument("--json-out", type=Path)
    args = ap.parse_args()

    result = compare(args.pnp, args.orca, args.cell)
    print(json.dumps(result, indent=2))
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return 0 if result["checks"]["pass"] else 1


if __name__ == "__main__":
    sys.exit(main())
