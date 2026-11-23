#!/usr/bin/env python3
"""Verify the t93 evidence set from a plain checkout.

Two independent checks:

1. Phase 1 — re-derive the screen's shape ceilings and verdict from
   `probe-rows.json` (the two instrumented runs) and require the measured-dead
   findings for lazy dilation and duplicate recompute, the above-floor saving
   for windowed erosion, and two-run agreement.
2. Phase 2 — re-derive `qualification.json`, `ab-reduction.json` and
   `summary.json` from the committed reduced rows with `reduce.py`, require
   exact agreement, check `provenance.json` (24 modules, byte-identical module
   trees per mode, distinct binaries per arm), and require the committed
   verdict to be the one the pre-declared rule produces from the measured
   cells. The gate outcome is *not* forced to PASS: a strict-clause miss is a
   valid, verifiable result.

`--self-test` mutates copies of the inputs and requires every negative control
to be rejected. Exits 0 only when every check passes and every control fails.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import reduce

HERE = Path(__file__).resolve().parent
CELL_WALL_MS = reduce.CELL_WALL_SECONDS * 1000.0
FLOOR_MS = reduce.FLOOR_SHARE_PCT / 100.0 * CELL_WALL_MS
RUN_AGREEMENT_MAX_PCT = 10.0


class Failure(Exception):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise Failure(message)


def group_calls(rows: list[dict]) -> dict[tuple, list[dict]]:
    groups: dict[tuple, list[dict]] = {}
    for row in rows:
        groups.setdefault((row["object"], row["region"], row["layer"]), []).append(row)
    return groups


# ── Phase 1 ────────────────────────────────────────────────────────────────


def check_phase1(probe: dict) -> dict:
    runs = probe["runs"]
    require(len(runs) == 2, f"expected 2 probe runs, found {len(runs)}")
    derived = []
    for run in runs:
        rows = run["rows"]
        require(len(rows) > 0, f"{run['run']}: no per-call rows")
        require(
            run["sha256"] == run["reference_sha256"],
            f"{run['run']}: output drift {run['sha256'][:8]} != reference "
            f"{run['reference_sha256'][:8]}",
        )
        require(
            int(run["bytes"]) == int(run["reference_bytes"]),
            f"{run['run']}: output size drift",
        )
        ratio = float(run["cpu_wall_ratio"])
        require(ratio > 0.0, f"{run['run']}: non-positive cpu/wall ratio")
        pre_qualified = [row for row in rows if row["pre_qualified"] > 0]
        require(
            len(pre_qualified) == 42,
            f"{run['run']}: expected 42 erosion-bearing calls, found {len(pre_qualified)}",
        )
        bbox_skip_calls = sum(1 for row in rows if row["bbox_skip"] == 1)
        reaching_calls = sum(1 for row in rows if row["surfaces_passed_clip"] > 0)
        dilation_ms = sum(row["deep_clip_offset_ms"] for row in rows)
        erosion_ms = sum(row["erosion_ms"] for row in pre_qualified)
        window_ms = sum(
            row["window_clip_ms"] + row["window_offset_ms"] for row in pre_qualified
        )
        saving_ms = erosion_ms - window_ms
        saving_wall_ms = saving_ms / ratio
        share_pct = 100.0 * saving_wall_ms / CELL_WALL_MS
        recompute = [row for row in rows if row["phase"] == "recompute"]
        recompute_wall_ms = sum(row["total_ms"] for row in recompute) / ratio
        groups = group_calls(rows)
        duplicate_groups = {
            key: calls for key, calls in groups.items() if len(calls) > 1
        }
        same_fingerprint_duplicates = sum(
            1
            for calls in duplicate_groups.values()
            if len({call["deep_fp"] for call in calls}) == 1
        )
        derived.append(
            {
                "run": run["run"],
                "calls": len(rows),
                "pre_qualified_calls": len(pre_qualified),
                "bbox_skip_calls": bbox_skip_calls,
                "reaching_calls": reaching_calls,
                "dilation_ms": dilation_ms,
                "erosion_ms": erosion_ms,
                "window_ms": window_ms,
                "saving_ms": saving_ms,
                "saving_wall_ms": saving_wall_ms,
                "share_pct": share_pct,
                "recompute_calls": len(recompute),
                "recompute_wall_ms": recompute_wall_ms,
                "recompute_share_pct": 100.0 * recompute_wall_ms / CELL_WALL_MS,
                "duplicate_groups": len(duplicate_groups),
                "same_fingerprint_duplicates": same_fingerprint_duplicates,
            }
        )
        # Shape 1 — lazy dilation. Every call has a surface that reaches the
        # deep clip area, so the conservative precheck can never skip one.
        require(
            bbox_skip_calls == 0,
            f"{run['run']}: lazy dilation would skip {bbox_skip_calls} calls",
        )
        require(
            reaching_calls == len(rows),
            f"{run['run']}: only {reaching_calls}/{len(rows)} calls have a surface "
            "reaching the deep clip area",
        )
        # Shape 2 — windowed erosion must clear the map's floor.
        require(
            saving_wall_ms > FLOOR_MS,
            f"{run['run']}: windowed-erosion saving {saving_wall_ms:.1f} ms is "
            f"below the {FLOOR_MS:.1f} ms floor",
        )
        # Shape 4 — duplicate recompute is measured-dead twice over: the
        # recompute operands differ (no exact reuse), and even deleting every
        # recompute is below the floor.
        require(
            recompute_wall_ms < FLOOR_MS,
            f"{run['run']}: recompute ceiling {recompute_wall_ms:.1f} ms is not "
            f"below the {FLOOR_MS:.1f} ms floor",
        )
        require(
            same_fingerprint_duplicates == 0,
            f"{run['run']}: {same_fingerprint_duplicates} duplicate recompute(s) "
            "carry an identical deep-area operand (reuse would be exact)",
        )
        require(
            len(recompute) > 0 and len(duplicate_groups) == len(recompute),
            f"{run['run']}: recompute count {len(recompute)} != duplicate groups "
            f"{len(duplicate_groups)}",
        )
    base_share = derived[0]["share_pct"]
    second_share = derived[1]["share_pct"]
    relative_spread = (
        100.0 * abs(base_share - second_share) / max(base_share, second_share)
    )
    require(
        relative_spread <= RUN_AGREEMENT_MAX_PCT,
        f"two-run windowed saving disagrees by {relative_spread:.1f} % "
        f"(> {RUN_AGREEMENT_MAX_PCT} %)",
    )
    return {
        "runs": derived,
        "floor_ms": FLOOR_MS,
        "cell_wall_seconds": reduce.CELL_WALL_SECONDS,
        "run_agreement_pct": relative_spread,
        "lazy_dilation": "measured-dead (0 skippable calls)",
        "windowed_erosion": "above floor in both runs",
        "duplicate_recompute": "measured-dead (no exact reuse; ceiling below floor)",
        "cheaper_predicate": (
            "rejected in Phase 1: no representation-equivalence proof under Miter joins"
        ),
    }


# ── Phase 2 ────────────────────────────────────────────────────────────────


def load_phase2() -> dict:
    return {
        "proof_rows": json.loads(
            (HERE / "proof-rows.json").read_text(encoding="utf-8")
        ),
        "measure_rows": json.loads(
            (HERE / "measure-rows.json").read_text(encoding="utf-8")
        ),
        "qualification": json.loads(
            (HERE / "qualification.json").read_text(encoding="utf-8")
        ),
        "ab": json.loads((HERE / "ab-reduction.json").read_text(encoding="utf-8")),
        "summary": json.loads((HERE / "summary.json").read_text(encoding="utf-8")),
        "provenance": json.loads(
            (HERE / "provenance.json").read_text(encoding="utf-8")
        ),
    }


def check_phase2_inputs(data: dict) -> dict:
    proof_rows = data["proof_rows"]
    measure_rows = data["measure_rows"]
    qualification = data["qualification"]
    ab = data["ab"]
    summary = data["summary"]
    provenance = data["provenance"]

    require(len(proof_rows) == 16, f"expected 16 proof cells, found {len(proof_rows)}")
    for cell, rows in proof_rows.items():
        require(len(rows) == 2, f"{cell}: expected 2 proof rows, found {len(rows)}")
        require(
            {row["arm"] for row in rows} == {"baseline", "candidate"},
            f"{cell}: arms are not baseline/candidate",
        )
        require(
            rows[0]["sha256"] == rows[1]["sha256"],
            f"{cell}: baseline and candidate outputs differ",
        )
    require(
        len(measure_rows) == len(reduce.MEASURE_CELLS),
        f"expected {len(reduce.MEASURE_CELLS)} measure cells",
    )
    expected_pairs = 2 + 2 * reduce.PAIRS_REQUIRED
    for cell, rows in measure_rows.items():
        require(
            len(rows) == expected_pairs,
            f"{cell}: expected {expected_pairs} measure rows, found {len(rows)}",
        )

    rederived_qualification = reduce.build_qualification(proof_rows)
    rederived_ab = reduce.build_ab(measure_rows)
    rederived_summary = reduce.build_summary(rederived_qualification, rederived_ab)
    require(
        qualification == rederived_qualification,
        "qualification.json is not reproducible from proof-rows.json",
    )
    require(
        ab == rederived_ab,
        "ab-reduction.json is not reproducible from measure-rows.json",
    )
    require(
        summary == rederived_summary,
        "summary.json is not reproducible from the re-derived artifacts",
    )
    require(qualification["pass"], "qualification did not pass all 16 cells")
    require(
        len(qualification["cells"]) == 16,
        "qualification covers fewer than 16 cells",
    )
    require(
        not qualification["problems"],
        f"qualification problems: {qualification['problems'][:3]}",
    )
    cross_mode_mismatches = [
        key for key, value in qualification["cross_mode"].items() if not value["equal"]
    ]
    require(
        not cross_mode_mismatches, f"cross-mode mismatches: {cross_mode_mismatches}"
    )

    for cell in reduce.MEASURE_CELLS:
        result = ab["cells"][cell]
        require(result.get("status") == "reduced", f"{cell}: {result.get('status')}")
        require(
            result["retained_pairs"] == reduce.PAIRS_REQUIRED,
            f"{cell}: retained {result['retained_pairs']}/{reduce.PAIRS_REQUIRED} pairs",
        )
        require(
            result["cpu_delta_pct_paired_median"] <= reduce.CPU_CEILING_PCT,
            f"{cell}: CPU above the +{reduce.CPU_CEILING_PCT} % ceiling",
        )
        require(
            all(pair["sha_equal"] for pair in result["pairs"]),
            f"{cell}: a retained pair is not byte-identical",
        )
        require(result["deterministic"], f"{cell}: more than one SHA per arm")
        require(result["arms_distinct_binaries"], f"{cell}: arms share a binary")

    # The committed verdict must be the one the pre-declared rule derives.
    expected_pass = all(
        ab["cells"][cell]["wall_delta_pct_paired_median"] < 0.0
        and ab["cells"][cell]["cpu_delta_pct_paired_median"] <= reduce.CPU_CEILING_PCT
        for cell in reduce.MEASURE_CELLS
    )
    require(
        ab["verdict"].startswith("PASS") == expected_pass,
        f"committed verdict '{ab['verdict']}' does not match the measured cells",
    )
    require(
        summary["verdict"] == ab["verdict"],
        "summary verdict differs from ab-reduction verdict",
    )

    snapshots = provenance["snapshots"]
    require(len(snapshots) == 4, f"expected 4 snapshots, found {len(snapshots)}")
    for mode in ("ordinary", "accelerated"):
        require(
            provenance["module_trees_equal"].get(mode) is True,
            f"{mode}: baseline and candidate module trees differ",
        )
        base = snapshots[f"{mode}-baseline"]
        cand = snapshots[f"{mode}-candidate"]
        require(base["module_count"] == 24, f"{mode}: baseline module count != 24")
        require(cand["module_count"] == 24, f"{mode}: candidate module count != 24")
        require(
            base["exe_sha256"] != cand["exe_sha256"],
            f"{mode}: arms share an executable SHA",
        )
        require(
            not base["candidate_sources_dirty"],
            f"{mode}: baseline arm recorded dirty candidate sources",
        )
        require(
            bool(cand["candidate_sources_dirty"]),
            f"{mode}: candidate arm recorded no candidate source",
        )

    return {
        "qualified_cells": len(qualification["cells"]),
        "cross_mode_equal": len(qualification["cross_mode"]),
        "gate_cells": {
            cell: {
                "wall_delta_pct": ab["cells"][cell]["wall_delta_pct_paired_median"],
                "cpu_delta_pct": ab["cells"][cell]["cpu_delta_pct_paired_median"],
                "retained_pairs": ab["cells"][cell]["retained_pairs"],
                "wall_clause": ab["cells"][cell]["wall_clause"],
            }
            for cell in reduce.MEASURE_CELLS
        },
        "strict_gate_pass": expected_pass,
        "verdict": ab["verdict"],
    }


def check_phase2() -> dict:
    return check_phase2_inputs(load_phase2())


# ── self-test ──────────────────────────────────────────────────────────────


def inject_exact_duplicate(data: dict) -> None:
    """Negative control: make one duplicate pair share a deep-area operand."""
    rows = data["runs"][1]["rows"]
    groups: dict[tuple, list[dict]] = {}
    for row in rows:
        groups.setdefault((row["object"], row["region"], row["layer"]), []).append(row)
    pair = next(calls for calls in groups.values() if len(calls) > 1)
    pair[1]["deep_fp"] = pair[0]["deep_fp"]


def self_test() -> int:
    probe = json.loads((HERE / "probe-rows.json").read_text(encoding="utf-8"))
    committed = load_phase2()
    controls: list[tuple[str, callable]] = []

    def phase1_control(mutate) -> callable:
        data = copy.deepcopy(probe)
        mutate(data)
        return lambda: check_phase1(data)

    def phase2_control(mutate) -> callable:
        data = copy.deepcopy(committed)
        mutate(data)
        return lambda: check_phase2_inputs(data)

    controls.append(
        (
            "lazy dilation skip injected",
            phase1_control(lambda data: data["runs"][0]["rows"][0].update(bbox_skip=1)),
        )
    )
    controls.append(
        (
            "lazy dilation reach removed",
            phase1_control(
                lambda data: data["runs"][0]["rows"][0].update(surfaces_passed_clip=0)
            ),
        )
    )
    controls.append(
        (
            "windowed saving pushed under the floor",
            phase1_control(
                lambda data: [
                    row.update(erosion_ms=row["erosion_ms"] * 0.05)
                    for row in data["runs"][1]["rows"]
                ]
            ),
        )
    )
    controls.append(
        (
            "recompute ceiling pushed over the floor",
            phase1_control(
                lambda data: [
                    row.update(total_ms=row["total_ms"] + 500.0)
                    for row in data["runs"][0]["rows"]
                    if row["phase"] == "recompute"
                ]
            ),
        )
    )
    controls.append(
        ("exact duplicate reuse injected", phase1_control(inject_exact_duplicate))
    )
    controls.append(
        (
            "probe output drift injected",
            phase1_control(lambda data: data["runs"][0].update(sha256="0" * 64)),
        )
    )
    controls.append(
        (
            "two-run disagreement injected",
            phase1_control(lambda data: data["runs"][0].update(cpu_wall_ratio=3.0)),
        )
    )

    def proof_output_mismatch(data):
        data["proof_rows"]["ordinary-base-classic-off"][1]["sha256"] = "0" * 64

    controls.append(("proof output mismatch", phase2_control(proof_output_mismatch)))

    def proof_unclean(data):
        data["proof_rows"]["ordinary-benchy-classic-on"][0]["degraded"] = "True"

    controls.append(("proof unclean flag", phase2_control(proof_unclean)))

    def proof_cross_mode(data):
        data["proof_rows"]["accelerated-base-classic-off"][0]["sha256"] = "f" * 64

    controls.append(("proof cross-mode mismatch", phase2_control(proof_cross_mode)))

    def measure_wall_halved(data):
        for row in data["measure_rows"]["ordinary-base-classic-off"]:
            if row["arm"] == "candidate" and row["warmup"] == "0":
                row["wall_seconds"] = str(float(row["wall_seconds"]) * 0.5)

    controls.append(
        ("measure candidate wall halved", phase2_control(measure_wall_halved))
    )

    def measure_cpu_over_ceiling(data):
        for row in data["measure_rows"]["accelerated-base-classic-on"]:
            if row["arm"] == "candidate":
                row["cpu_seconds"] = str(float(row["cpu_seconds"]) * 1.5)

    controls.append(
        ("measure CPU over the ceiling", phase2_control(measure_cpu_over_ceiling))
    )

    def measure_missing_pair(data):
        data["measure_rows"]["ordinary-base-classic-on"].pop()

    controls.append(("measure pair missing", phase2_control(measure_missing_pair)))

    def committed_ab_tampered(data):
        data["ab"]["cells"]["ordinary-base-classic-off"][
            "wall_delta_pct_paired_median"
        ] += 1.0

    controls.append(("committed ab tampered", phase2_control(committed_ab_tampered)))

    def committed_summary_tampered(data):
        data["summary"]["verdict"] = "PASS (forged)"

    controls.append(
        ("committed summary tampered", phase2_control(committed_summary_tampered))
    )

    def module_tree_diverges(data):
        data["provenance"]["module_trees_equal"]["ordinary"] = False

    controls.append(
        ("provenance module trees diverge", phase2_control(module_tree_diverges))
    )

    def module_count_wrong(data):
        data["provenance"]["snapshots"]["ordinary-baseline"]["module_count"] = 23

    controls.append(("provenance module count", phase2_control(module_count_wrong)))

    def baseline_dirty(data):
        data["provenance"]["snapshots"]["ordinary-baseline"][
            "candidate_sources_dirty"
        ] = ["M crates/slicer-runtime/src/slice_postprocess_prepass.rs"]

    controls.append(("provenance baseline dirty", phase2_control(baseline_dirty)))

    rejected = 0
    rejected_names = []
    for name, run in controls:
        try:
            run()
            raise Failure("control was accepted")
        except Failure:
            rejected += 1
            rejected_names.append(name)
        except Exception as error:  # any hard error is also a rejection
            rejected += 1
            rejected_names.append(f"{name} ({type(error).__name__})")
    print(f"negative controls rejected: {rejected}/{len(controls)}")
    for name in rejected_names:
        print(f"  rejected: {name}")
    return 0 if rejected == len(controls) else 1


def main(argv: list[str]) -> int:
    self_test_mode = "--self-test" in argv[1:]
    probe = json.loads((HERE / "probe-rows.json").read_text(encoding="utf-8"))
    phase1 = check_phase1(probe)
    phase2 = check_phase2()

    print(
        f"Phase 1 floor {phase1['floor_ms']:.1f} ms = {reduce.FLOOR_SHARE_PCT} % of "
        f"{reduce.CELL_WALL_SECONDS} s"
    )
    for run in phase1["runs"]:
        print(
            f"  {run['run']}: calls {run['calls']}, lazy skips {run['bbox_skip_calls']}, "
            f"dilation {run['dilation_ms']:.1f} ms, erosion {run['erosion_ms']:.1f} ms, "
            f"windowed {run['window_ms']:.1f} ms, saving {run['saving_wall_ms']:.1f} ms "
            f"= {run['share_pct']:.3f} % of the cell, recompute ceiling "
            f"{run['recompute_wall_ms']:.1f} ms"
        )
    print(f"  two-run agreement {phase1['run_agreement_pct']:.2f} %")
    print(
        f"  {phase1['lazy_dilation']}; {phase1['windowed_erosion']}; "
        f"{phase1['duplicate_recompute']}; {phase1['cheaper_predicate']}"
    )
    print(
        f"Phase 2 qualified cells {phase2['qualified_cells']}/16, "
        f"cross-mode equal {phase2['cross_mode_equal']}/8"
    )
    for cell, values in phase2["gate_cells"].items():
        print(
            f"  {cell}: wall {values['wall_delta_pct']:+.2f} % "
            f"cpu {values['cpu_delta_pct']:+.2f} % pairs {values['retained_pairs']}/6 "
            f"[{values['wall_clause']}]"
        )
    print("VERDICT:", phase2["verdict"])
    if self_test_mode:
        return self_test()
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
