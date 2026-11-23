#!/usr/bin/env python3
"""Checkout-only verifier for the t78 planner emit-pass batching take.

Three committed artifacts, each with a negative-control suite:

- `summary.json` — the take's first-cell observation (one run per arm) and the
  same-instrument stage attribution.
- `qualification.json` — the 16-cell output-identity matrix.
- `ab-reduction.json` — the 6-pair alternating campaign on the four
  pre-declared classic supports-on cells.

The raw G-code and stderr captures are large and live under `target/` and
`.local-artifacts/`; the committed files are the reduced set that
`run-cell.ps1`, `build-qualification.py` and `reduce_ab.py` produce.

Exits 0 only if every check passes and every negative control is rejected.
"""

from __future__ import annotations

import copy
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SUMMARY = HERE / "summary.json"
QUALIFICATION = HERE / "qualification.json"
AB = HERE / "ab-reduction.json"
PATCH = HERE / "candidate.patch"

#: Preserved t58 reference for base classic supports-on (t77's verifier pins the
#: same value for the same cell).
T58_REFERENCE_SHA = "f60e5e9b8c0d836cf73170e645c18f9193b91d47e694d335cfb1d78032682319"

MODES = ("ordinary", "accelerated")
FIXTURES = ("benchy", "base")
GENERATORS = ("classic", "arachne")
SUPPORTS = ("off", "on")
ALL_CELLS = {
    f"{mode}-{fixture}-{gen}-{sup}"
    for mode in MODES
    for fixture in FIXTURES
    for gen in GENERATORS
    for sup in SUPPORTS
}
GATE_CELLS = {
    f"{mode}-{fixture}-classic-on" for mode in MODES for fixture in FIXTURES
}
CPU_CEILING_PCT = 3.0


class CheckFailed(Exception):
    pass


def check(condition: bool, message: str) -> None:
    if not condition:
        raise CheckFailed(message)


def verify_summary(summary: dict) -> None:
    arms = summary["arms"]
    derived = summary["derived"]
    attribution = summary["attribution"]

    # 1. Both arms are the preserved reference output, byte for byte. The take's
    #    exactness claim is identity, not a geometric tolerance.
    for arm in ("baseline", "candidate"):
        check(
            arms[arm]["sha256"] == T58_REFERENCE_SHA,
            f"{arm} output sha {arms[arm]['sha256']} != t58 reference",
        )
        check(
            arms[arm]["bytes"] == summary["reference_bytes"],
            f"{arm} output bytes {arms[arm]['bytes']} != {summary['reference_bytes']}",
        )

    # 2. The candidate wall falls, and the recorded delta is the arms' delta.
    wall_delta = arms["candidate"]["wall_seconds"] - arms["baseline"]["wall_seconds"]
    check(wall_delta < 0.0, f"candidate wall delta {wall_delta:.3f}s is not a fall")
    check(
        abs(derived["wall_delta_seconds"] - wall_delta) <= 0.002,
        f"recorded wall delta {derived['wall_delta_seconds']:.3f}s != re-derived {wall_delta:.3f}s",
    )
    wall_percent = 100.0 * wall_delta / arms["baseline"]["wall_seconds"]
    check(
        abs(derived["wall_delta_percent"] - wall_percent) <= 0.01,
        f"recorded wall delta {derived['wall_delta_percent']:.3f}% != re-derived {wall_percent:.3f}%",
    )

    # 3. CPU corroborates rather than contradicts: the batching moves the same
    #    work onto more cores, so total process CPU must stay in a small band
    #    (marshalling overhead allowed, a disappearing workload not).
    cpu_delta = 100.0 * (
        arms["candidate"]["cpu_seconds"] / arms["baseline"]["cpu_seconds"] - 1.0
    )
    check(
        abs(derived["cpu_delta_percent"] - cpu_delta) <= 0.01,
        f"recorded CPU delta {derived['cpu_delta_percent']:.3f}% != re-derived {cpu_delta:.3f}%",
    )
    check(
        -2.0 <= cpu_delta <= CPU_CEILING_PCT,
        f"candidate CPU delta {cpu_delta:.3f}% is outside the work-preserving band",
    )
    check(
        arms["candidate"]["cpu_wall_ratio"] > arms["baseline"]["cpu_wall_ratio"],
        "candidate must occupy more cores concurrently than the serial baseline",
    )

    # 4. The attribution pair names the mechanism.
    baseline = attribution["baseline"]
    candidate = attribution["candidate"]
    for arm in (baseline, candidate):
        check(
            arm["planner_module_ms"] <= arm["support_geometry_ms"],
            f"planner module {arm['planner_module_ms']}ms exceeds its stage "
            f"{arm['support_geometry_ms']}ms",
        )
    stage_drop = candidate["support_geometry_ms"] - baseline["support_geometry_ms"]
    module_drop = candidate["planner_module_ms"] - baseline["planner_module_ms"]
    check(stage_drop < 0, f"SupportGeometry stage delta {stage_drop}ms is not a fall")
    check(module_drop < 0, f"planner module delta {module_drop}ms is not a fall")
    check(
        abs(derived["stage_drop_ms"] - stage_drop) <= 1.0,
        f"recorded stage drop {derived['stage_drop_ms']}ms != re-derived {stage_drop}ms",
    )
    check(
        abs(derived["module_drop_ms"] - module_drop) <= 1.0,
        f"recorded module drop {derived['module_drop_ms']}ms != re-derived {module_drop}ms",
    )
    check(
        stage_drop <= baseline["support_geometry_ms"],
        "stage drop exceeds the baseline stage wall",
    )

    # 5. The unit parity gate ran and passed.
    tests = summary["unit_tests"]
    check(tests["failed"] == 0, f"{tests['failed']} module tests failed")
    check(tests["passed"] >= 62, f"only {tests['passed']} module tests passed")


def verify_patch() -> None:
    text = PATCH.read_text(encoding="utf-8")
    check("build_roles_batched" in text, "candidate.patch does not add build_roles_batched")
    files = [line for line in text.splitlines() if line.startswith("diff --git ")]
    check(len(files) == 1, f"candidate.patch touches {len(files)} files, expected 1")
    check(
        "modules/core-modules/tree-support-planner/src/lib.rs" in files[0],
        "candidate.patch does not target the tree-support planner guest",
    )
    check(
        "largest_part" in text,
        "candidate.patch does not share the carve survivor between both paths",
    )


def verify_qualification(qualification: dict) -> None:
    check(qualification["pass"] is True, "qualification did not pass")
    cells = qualification["cells"]
    check(len(cells) == 16, f"qualification holds {len(cells)} cells, expected 16")
    names = {c["cell"] for c in cells}
    check(names == ALL_CELLS, f"qualification cell set != the 16-cell matrix")
    check(
        qualification["problems"] == [],
        f"qualification recorded {len(qualification['problems'])} problems",
    )
    for cell in cells:
        where = cell["cell"]
        check(
            cell["sha256"] == cell["reference_sha256"],
            f"{where}: output {cell['sha256'][:8]} != preserved t58 reference "
            f"{cell['reference_sha256'][:8]}",
        )
        check(len(cell["sha256"]) == 64, f"{where}: sha is not a 64-char hex digest")
        check(int(cell["bytes"]) > 0, f"{where}: empty output recorded")
        check(
            set(cell["wall_seconds"]) == {"baseline", "candidate"},
            f"{where}: both arms must be recorded",
        )
        check(
            all(float(v) > 0.0 for v in cell["wall_seconds"].values())
            and all(float(v) > 0.0 for v in cell["cpu_seconds"].values()),
            f"{where}: non-positive timing recorded",
        )
        check(cell["status"] == "ok", f"{where}: status {cell['status']}")
        check(cell["degraded"] == "False", f"{where}: output degraded")


def _paired_median(rows: list[dict], field_b: str, field_c: str) -> float:
    ratios = [float(r[field_c]) / float(r[field_b]) for r in rows]
    return (statistics.median(ratios) - 1.0) * 100.0


def verify_campaign(ab: dict) -> None:
    cells = ab["cells"]
    check(
        set(cells) == GATE_CELLS,
        f"campaign cells {sorted(cells)} != pre-declared gate {sorted(GATE_CELLS)}",
    )
    check(ab["cpu_clause_ceiling_pct"] == CPU_CEILING_PCT, "CPU ceiling drifted")
    for name in sorted(GATE_CELLS):
        cell = cells[name]
        check(cell.get("status") == "reduced", f"{name}: not reduced")
        check(
            cell["retained_pairs"] == 6,
            f"{name}: {cell['retained_pairs']} retained pairs, gate requires 6",
        )
        check(cell["excluded_pairs"] == [], f"{name}: starved pairs excluded")
        check(
            cell["wall_delta_pct_paired_median"] < 0.0,
            f"{name}: wall delta {cell['wall_delta_pct_paired_median']:+.2f}% is not a fall",
        )
        check(
            cell["cpu_delta_pct_paired_median"] <= CPU_CEILING_PCT,
            f"{name}: CPU delta {cell['cpu_delta_pct_paired_median']:+.2f}% over ceiling",
        )
        pairs = cell["pairs"]
        check(len(pairs) == 6, f"{name}: {len(pairs)} pairs, expected 6")
        check(
            all(p["sha_equal"] for p in pairs),
            f"{name}: a retained pair's arms are not byte-identical",
        )
        check(
            sorted(p["pair"] for p in pairs) == [1, 2, 3, 4, 5, 6],
            f"{name}: pair indices are not 1..6",
        )
        wall = _paired_median(pairs, "baseline_wall_s", "candidate_wall_s")
        check(
            abs(wall - cell["wall_delta_pct_paired_median"]) <= 0.01,
            f"{name}: recorded wall median {cell['wall_delta_pct_paired_median']:+.3f}% "
            f"!= re-derived {wall:+.3f}%",
        )
        cpu = _paired_median(pairs, "baseline_cpu_s", "candidate_cpu_s")
        check(
            abs(cpu - cell["cpu_delta_pct_paired_median"]) <= 0.01,
            f"{name}: recorded CPU median {cell['cpu_delta_pct_paired_median']:+.3f}% "
            f"!= re-derived {cpu:+.3f}%",
        )
    check(
        ab["verdict"].startswith("PASS"),
        f"verdict is not a pass: {ab['verdict']}",
    )


def main() -> int:
    summary = json.loads(SUMMARY.read_text(encoding="utf-8"))
    qualification = json.loads(QUALIFICATION.read_text(encoding="utf-8"))
    ab = json.loads(AB.read_text(encoding="utf-8"))
    try:
        verify_summary(summary)
        verify_patch()
        verify_qualification(qualification)
        verify_campaign(ab)
    except CheckFailed as error:
        print(f"FAIL: {error}")
        return 1

    mutators: dict[str, tuple[object, callable]] = {
        "summary: baseline output drift": (
            summary,
            lambda s: s["arms"]["baseline"].__setitem__("sha256", "0" * 64),
        ),
        "summary: candidate wall above baseline": (
            summary,
            lambda s: s["arms"]["candidate"].__setitem__(
                "wall_seconds", s["arms"]["baseline"]["wall_seconds"] + 1.0
            ),
        ),
        "summary: stale wall delta": (
            summary,
            lambda s: s["derived"].__setitem__(
                "wall_delta_seconds", s["derived"]["wall_delta_seconds"] + 5.0
            ),
        ),
        "summary: CPU that contradicts": (
            summary,
            lambda s: s["derived"].__setitem__("cpu_delta_percent", 40.0),
        ),
        "summary: serial core occupancy": (
            summary,
            lambda s: s["arms"]["candidate"].__setitem__(
                "cpu_wall_ratio", s["arms"]["baseline"]["cpu_wall_ratio"] - 0.5
            ),
        ),
        "summary: stage that grew": (
            summary,
            lambda s: s["attribution"]["candidate"].__setitem__(
                "support_geometry_ms",
                s["attribution"]["baseline"]["support_geometry_ms"] + 1,
            ),
        ),
        "summary: module outside its stage": (
            summary,
            lambda s: s["attribution"]["candidate"].__setitem__(
                "planner_module_ms",
                s["attribution"]["candidate"]["support_geometry_ms"] + 1,
            ),
        ),
        "summary: stale stage drop": (
            summary,
            lambda s: s["derived"].__setitem__(
                "stage_drop_ms", s["derived"]["stage_drop_ms"] + 10
            ),
        ),
        "summary: failing unit test": (
            summary,
            lambda s: s["unit_tests"].__setitem__("failed", 1),
        ),
        "qualification: gate not passed": (
            qualification,
            lambda q: q.__setitem__("pass", False),
        ),
        "qualification: output drift in one cell": (
            qualification,
            lambda q: q["cells"][0].__setitem__("sha256", "1" * 64),
        ),
        "qualification: missing cell": (
            qualification,
            lambda q: q["cells"].pop(),
        ),
        "qualification: degraded cell": (
            qualification,
            lambda q: q["cells"][0].__setitem__("degraded", "True"),
        ),
        "campaign: wall clause fails in one cell": (
            ab,
            lambda a: a["cells"]["ordinary-base-classic-on"].__setitem__(
                "wall_delta_pct_paired_median", 0.5
            ),
        ),
        "campaign: retained pair is not byte-identical": (
            ab,
            lambda a: a["cells"]["ordinary-base-classic-on"]["pairs"][0].__setitem__(
                "sha_equal", False
            ),
        ),
        "campaign: starved pair excluded": (
            ab,
            lambda a: a["cells"]["accelerated-benchy-classic-on"].__setitem__(
                "excluded_pairs", [3]
            ),
        ),
        "campaign: CPU over ceiling": (
            ab,
            lambda a: a["cells"]["accelerated-base-classic-on"].__setitem__(
                "cpu_delta_pct_paired_median", CPU_CEILING_PCT + 1.0
            ),
        ),
        "campaign: only five retained pairs": (
            ab,
            lambda a: a["cells"]["ordinary-benchy-classic-on"].__setitem__(
                "retained_pairs", 5
            ),
        ),
        "campaign: verdict not a pass": (
            ab,
            lambda a: a.__setitem__("verdict", "GATE NOT MET"),
        ),
    }
    verifiers = {
        "summary": verify_summary,
        "qualification": verify_qualification,
        "campaign": verify_campaign,
    }
    rejected = 0
    for name, (document, mutate) in mutators.items():
        mutated = copy.deepcopy(document)
        mutate(mutated)
        verifier = verifiers[name.split(":")[0]]
        try:
            verifier(mutated)
        except CheckFailed:
            rejected += 1
        else:
            print(f"FAIL: negative control not rejected: {name}")
            return 1

    print(f"OK: all checks pass; {rejected}/{len(mutators)} negative controls rejected")
    return 0


if __name__ == "__main__":
    sys.exit(main())
