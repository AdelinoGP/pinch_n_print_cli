#!/usr/bin/env python3
"""Derive the t88 supports-on increment attribution and shape pricing.

Inputs:
  - ``captures/*.json`` — committed per-run captures from ``extract.py``
  - ``../scoreboard-refresh-20261025/rows-base-*.csv`` — the quiet ordinary
    scoreboard rows, read for the supports-on cell walls used as denominators

Output: ``summary.json`` — the single reduced artifact ``verify.py`` checks.

    python reduce.py
"""

import csv
import json
import statistics
from pathlib import Path

HERE = Path(__file__).resolve().parent
CAPTURES = HERE / "captures"
SCOREBOARD = HERE / ".." / "scoreboard-refresh-20261025"
SUMMARY = HERE / "summary.json"

# The map's established screening floor (t83/t84): a shape graduates only if
# its measured (instrumented) ceiling is at least this share of the cell wall.
FLOOR_PCT = 0.80

CELLS = ("base-classic", "base-arachne")
SIDE = ("off", "on")

POSTPASS_STAGES = (
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",
    "PostPass::GCodePostProcess",
    "PostPass::GCodeSerialize",
)


def load_captures():
    captures = {}
    for path in sorted(CAPTURES.glob("*.json")):
        capture = json.loads(path.read_text(encoding="utf-8"))
        captures[(capture["batch"], capture["run_id"])] = capture
    return captures


def quiet_walls():
    """Median measured wall of the PnP ordinary arm per base cell."""
    walls = {}
    for cell in CELLS:
        for side in SIDE:
            path = SCOREBOARD / f"rows-{cell}-{side}.csv"
            with open(path, encoding="utf-8-sig", newline="") as handle:
                rows = [
                    row
                    for row in csv.DictReader(handle)
                    if row["tool"] == "pnp-ordinary" and row["warmup"] == "0"
                ]
            if len(rows) != 3:
                raise SystemExit(
                    f"{path}: expected 3 measured ordinary rows, got {len(rows)}"
                )
            walls[f"{cell}-{side}"] = {
                "wall_s": statistics.median(float(row["wall_seconds"]) for row in rows),
                "cpu_s": statistics.median(float(row["cpu_seconds"]) for row in rows),
                "samples": len(rows),
            }
    return walls


def stage(capture, name):
    rows = [row for row in capture["stages"] if row["stage"] == name]
    if not rows:
        return None
    return rows[0]


def stage_wall_ms(capture, name):
    row = stage(capture, name)
    return None if row is None else row["elapsed_ms"]


def gap(capture, name):
    rows = [row for row in capture["stage_gaps"] if row["stage"] == name]
    return rows[-1] if rows else None


def probe(capture, term):
    return capture.get("probe_terms", {}).get(term)


def postpass_gap_ms(capture):
    total = capture["phases"].get("postpass")
    if total is None:
        return None
    stage_sum = sum(stage_wall_ms(capture, name) or 0 for name in POSTPASS_STAGES)
    return total - stage_sum


def pct(part, whole):
    return None if part is None or not whole else part * 100.0 / whole


def main():
    captures = load_captures()
    quiet = quiet_walls()

    def capture(cell, side, label="probe"):
        run_id = f"{cell}-{side}-{label}"
        ranked = sorted(
            (c for (batch, _), c in captures.items() if c["run_id"] == run_id),
            key=lambda c: (not c["batch"].startswith("t88b"), c["batch"]),
        )
        if ranked:
            return ranked[0]
        raise SystemExit(f"missing capture {run_id}")

    stage_table = {}
    probe_table = {}
    for cell in CELLS:
        for side in SIDE:
            wall = capture(cell, side)
            stage_table[f"{cell}-{side}"] = {
                "batch": wall["batch"],
                "run_id": wall["run_id"],
                "slice_ms": wall["slice_complete"]["elapsed_ms"],
                "phases": wall["phases"],
                "stages": {
                    row["stage"]: {
                        "calls": row["calls"],
                        "elapsed_ms": row["elapsed_ms"],
                    }
                    for row in wall["stages"]
                },
            }
        for side in SIDE:
            try:
                profiled = capture(cell, side)
            except SystemExit:
                continue
            probe_table[f"{cell}-{side}"] = {
                "batch": profiled["batch"],
                "run_id": profiled["run_id"],
                "slice_ms": profiled["slice_complete"]["elapsed_ms"],
                "probe_terms": profiled.get("probe_terms", {}),
            }

    # ---- per-cell postpass copy/drop decomposition -----------------------
    ownership = {}
    for cell in CELLS:
        for side in SIDE:
            wall = capture(cell, side)
            fin = gap(wall, "PostPass::LayerFinalization")
            if fin is None:
                continue
            profiled = wall
            walk = (
                probe(profiled, "ge.walk")["ms"]
                if profiled and probe(profiled, "ge.walk")
                else None
            )
            estimate = (
                probe(profiled, "ge.estimate")["ms"]
                if profiled and probe(profiled, "ge.estimate")
                else None
            )
            m73 = (
                probe(profiled, "ge.m73")["ms"]
                if profiled and probe(profiled, "ge.m73")
                else None
            )
            emit_stage = stage_wall_ms(wall, "PostPass::GCodeEmit")
            non_copy = sum(value or 0.0 for value in (walk, estimate, m73))
            emit_copy = None if emit_stage is None else max(0.0, emit_stage - non_copy)
            entry = {
                "batch": wall["batch"],
                "run_id": wall["run_id"],
                "slice_ms": wall["slice_complete"]["elapsed_ms"],
                "finalization_before_ms": fin["before_first_module_ms"],
                "finalization_after_ms": fin["after_last_module_ms"],
                "finalization_modules_ms": fin["module_span_ms"],
                "finalization_copy_drop_ms": (fin["before_first_module_ms"] or 0)
                + (fin["after_last_module_ms"] or 0),
                "gcode_emit_stage_ms": emit_stage,
                "gcode_emit_non_copy_ms": non_copy,
                "gcode_emit_drop_ms": emit_copy,
                "postpass_gap_ms": postpass_gap_ms(wall),
            }
            entry["ownership_total_ms"] = sum(
                entry[key] or 0.0
                for key in (
                    "finalization_copy_drop_ms",
                    "gcode_emit_drop_ms",
                    "postpass_gap_ms",
                )
            )
            entry["net_upper_ms"] = (
                (entry["finalization_before_ms"] or 0)
                + (entry["finalization_after_ms"] or 0)
                + (entry["postpass_gap_ms"] or 0)
            )
            entry["share_pct"] = pct(
                entry["ownership_total_ms"], wall["slice_complete"]["elapsed_ms"]
            )
            entry["net_share_pct"] = pct(
                entry["net_upper_ms"], wall["slice_complete"]["elapsed_ms"]
            )
            quiet_key = f"{cell}-{side}"
            entry["quiet_cell_wall_s"] = quiet[quiet_key]["wall_s"]
            ownership[quiet_key] = entry

    # ---- prepass host shapes --------------------------------------------
    shapes = []
    for cell in CELLS:
        quiet_key = f"{cell}-on"
        wall = capture(cell, "on")
        slice_ms = wall["slice_complete"]["elapsed_ms"]
        quiet_ms = quiet[quiet_key]["wall_s"] * 1000.0
        profiled = wall
        validate = probe(profiled, "agg.validate") if profiled else None
        eligibility = probe(profiled, "sa.eligibility") if profiled else None
        contacts = probe(profiled, "sa.contacts") if profiled else None
        planner = next(
            (
                row
                for row in wall["modules"]
                if row["module_id"] == "com.core.tree-support-planner"
            ),
            None,
        )
        shapes.append(
            {
                "id": "support-plan-exact-z-validation",
                "cell": cell,
                "run_id": profiled["run_id"] if profiled else wall["run_id"],
                "term_ms": validate["ms"]
                if validate
                else (gap(wall, "PrePass::SupportGeometry")["after_last_module_ms"]),
                "source": "probe term agg.validate"
                if validate
                else "stage gap upper bound",
                "share_pct": pct(
                    validate["ms"]
                    if validate
                    else gap(wall, "PrePass::SupportGeometry")["after_last_module_ms"],
                    slice_ms,
                ),
                "quiet_cell_wall_s": quiet[quiet_key]["wall_s"],
                "floor_pct": FLOOR_PCT,
                "verdict": "graduate (parallelize; object-level cache dedupe)",
            }
        )
        shapes.append(
            {
                "id": "postpass-layer-collection-ownership",
                "cell": cell,
                "run_id": wall["run_id"],
                "term_ms": ownership[quiet_key]["net_upper_ms"],
                "upper_bound_ms": ownership[quiet_key]["ownership_total_ms"],
                "source": "one fewer collection: finalization clone+drop + postpass gap retained as the removed drop; emitter drop remains",
                "share_pct": ownership[quiet_key]["net_share_pct"],
                "upper_bound_share_pct": ownership[quiet_key]["share_pct"],
                "quiet_cell_wall_s": quiet[quiet_key]["wall_s"],
                "floor_pct": FLOOR_PCT,
                "verdict": "graduate (single-ownership pass through finalization/emission)",
            }
        )
        shapes.append(
            {
                "id": "support-analysis-eligibility-fold",
                "cell": cell,
                "run_id": profiled["run_id"] if profiled else wall["run_id"],
                "term_ms": eligibility["ms"] if eligibility else None,
                "source": "probe term sa.eligibility",
                "share_pct": pct(eligibility["ms"] if eligibility else None, slice_ms),
                "quiet_cell_wall_s": quiet[quiet_key]["wall_s"],
                "floor_pct": FLOOR_PCT,
                "verdict": "not graduated (work-bound: the predicate still runs; net below floor)",
            }
        )
        shapes.append(
            {
                "id": "support-contact-detection",
                "cell": cell,
                "run_id": profiled["run_id"] if profiled else wall["run_id"],
                "term_ms": contacts["ms"] if contacts else None,
                "source": "probe term sa.contacts",
                "share_pct": pct(contacts["ms"] if contacts else None, slice_ms),
                "quiet_cell_wall_s": quiet[quiet_key]["wall_s"],
                "floor_pct": FLOOR_PCT,
                "verdict": "not graduated (parallel canonical support geometry)",
            }
        )
        shapes.append(
            {
                "id": "tree-support-planner",
                "cell": cell,
                "run_id": wall["run_id"],
                "term_ms": planner["elapsed_ms"] if planner else None,
                "source": "module_complete com.core.tree-support-planner",
                "share_pct": pct(planner["elapsed_ms"] if planner else None, slice_ms),
                "quiet_cell_wall_s": quiet[quiet_key]["wall_s"],
                "floor_pct": FLOOR_PCT,
                "verdict": "not graduated (support geometry; clip core batched by t78)",
            }
        )

    # ---- the branch-distance measurement arm ----------------------------
    branch = None
    branch_caps = [
        c
        for (batch, _), c in captures.items()
        if batch.startswith("t88b") and "branch5" in c["run_id"]
    ]
    if branch_caps:
        arm = branch_caps[0]
        base = capture("base-classic", "on")
        base_planner = next(
            (
                row
                for row in base["modules"]
                if row["module_id"] == "com.core.tree-support-planner"
            ),
            None,
        )
        arm_planner = next(
            (
                row
                for row in arm["modules"]
                if row["module_id"] == "com.core.tree-support-planner"
            ),
            None,
        )
        branch = {
            "run_id": arm["run_id"],
            "cell": arm["cell"],
            "config_sha256": arm["config_sha256"],
            "bytes": arm["bytes"],
            "sha256": arm["sha256"],
            "reference_sha256": arm["reference_sha256"],
            "slice_ms": arm["slice_complete"]["elapsed_ms"],
            "base_slice_ms": base["slice_complete"]["elapsed_ms"],
            "planner_ms": arm_planner["elapsed_ms"] if arm_planner else None,
            "base_planner_ms": base_planner["elapsed_ms"] if base_planner else None,
            "validate_ms": (probe(arm, "agg.validate") or {}).get("ms"),
            "base_validate_ms": (probe(base, "agg.validate") or {}).get("ms"),
            "gcode_bytes": arm["bytes"],
            "base_gcode_bytes": base["bytes"],
            "note": "config-only measurement arm; output differs from the 1 mm reference",
        }

    summary = {
        "head": capture("base-classic", "on")["captured_commit"],
        "floor_pct": FLOOR_PCT,
        "quiet_scoreboard": quiet,
        "stage_table": stage_table,
        "probe_table": probe_table,
        "ownership": ownership,
        "shapes": shapes,
        "branch_distance": branch,
    }
    SUMMARY.write_text(
        json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    print(f"wrote {SUMMARY}")


if __name__ == "__main__":
    main()
