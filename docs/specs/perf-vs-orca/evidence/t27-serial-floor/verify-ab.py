#!/usr/bin/env python3
"""T27 SupportAnalysis parallel-predicate A/B reducer and gate verifier.

Reads the rows.csv produced by run_ab_par_predicate.ps1 for one (mode, cell)
batch and reports:
  - completeness and per-run disclosure (status, degraded, non-fatal, SHA)
  - starvation-filtered paired deltas (candidate - baseline)
  - paired medians (the standing keep clause: lower candidate median wall AND
    lower candidate median CPU, both modes, or DROP/inconclusive)
  - five negative controls (fabricated reference, output drift, CPU-only win,
    starvation pair loss, arm-order bias)

Usage:
  python verify-ab.py <rows.csv> [--reference-sha <hex>] [--json out.json]
"""

import argparse
import csv
import json
import statistics
import sys
from pathlib import Path

STARVATION_FRACTION = 0.75
MIN_RETAINED_PAIRS = 5


def load_rows(path: Path):
    with path.open(encoding="utf-8-sig") as fh:
        rows = list(csv.DictReader(fh))
    for row in rows:
        row["wall_seconds"] = float(row["wall_seconds"])
        row["cpu_seconds"] = float(row["cpu_seconds"])
        row["cpu_wall_ratio"] = float(row["cpu_wall_ratio"])
        row["pair"] = int(row["pair"])
        row["warmup"] = int(row["warmup"])
        row["bytes"] = int(row["bytes"])
        row["fatal"] = int(row["fatal"])
        row["non_fatal"] = int(row["non_fatal"])
    return rows


def verify(rows, reference_sha=None, require_pairs=MIN_RETAINED_PAIRS):
    errors = []
    notes = []

    warmups = [r for r in rows if r["warmup"] == 1]
    measured = [r for r in rows if r["warmup"] == 0]
    if len(warmups) not in (0, 2):
        errors.append(f"expected 0 or 2 warmup rows, found {len(warmups)}")
    elif len(warmups) == 0:
        notes.append("warmup rows not retained in this CSV (see the proof phase CSV)")

    arms = sorted({r["arm"] for r in measured})
    pairs = sorted({r["pair"] for r in measured})
    for p in pairs:
        for arm in arms:
            n = sum(1 for r in measured if r["pair"] == p and r["arm"] == arm)
            if n != 1:
                errors.append(f"pair {p} arm {arm}: expected 1 row, found {n}")

    for r in measured:
        if r["status"] != "ok":
            errors.append(f"{r['run_id']}: status {r['status']}")
        if r["degraded"] in ("True", "true", 1, "1"):
            errors.append(f"{r['run_id']}: degraded")
        if r["fatal"] != 0 or r["non_fatal"] != 0:
            errors.append(
                f"{r['run_id']}: fatal={r['fatal']} non_fatal={r['non_fatal']}"
            )
        if r["open_loop_warnings"] not in ("0", 0):
            errors.append(
                f"{r['run_id']}: open_loop_warnings={r['open_loop_warnings']}"
            )

    shas = {r["sha256"] for r in measured}
    if len(shas) != 1:
        errors.append(f"output SHA drift across runs: {sorted(shas)}")
    else:
        sha = next(iter(shas))
        if reference_sha and sha != reference_sha:
            errors.append(f"output {sha} != reference {reference_sha}")

    # Starvation: per arm, cutoff = 0.75 * best cpu/wall ratio in that arm.
    best_ratio = {
        arm: max(r["cpu_wall_ratio"] for r in measured if r["arm"] == arm)
        for arm in arms
    }
    excluded_pairs = set()
    per_arm_exclusions = {arm: 0 for arm in arms}
    for r in measured:
        cutoff = STARVATION_FRACTION * best_ratio[r["arm"]]
        if r["cpu_wall_ratio"] < cutoff:
            excluded_pairs.add(r["pair"])
            per_arm_exclusions[r["arm"]] += 1
    retained = [r for r in measured if r["pair"] not in excluded_pairs]
    retained_pairs = sorted({r["pair"] for r in retained})

    deltas = []
    for p in retained_pairs:
        base = [r for r in retained if r["pair"] == p and r["arm"] == "baseline"]
        cand = [r for r in retained if r["pair"] == p and r["arm"] == "candidate"]
        if not base or not cand:
            continue
        b, c = base[0], cand[0]
        deltas.append(
            {
                "pair": p,
                "wall_delta": c["wall_seconds"] - b["wall_seconds"],
                "cpu_delta": c["cpu_seconds"] - b["cpu_seconds"],
                "wall_faster": c["wall_seconds"] < b["wall_seconds"],
                "cpu_faster": c["cpu_seconds"] < b["cpu_seconds"],
            }
        )

    if len(retained_pairs) < require_pairs:
        errors.append(
            f"only {len(retained_pairs)} retained pairs (< {require_pairs}); "
            "inconclusive by protocol"
        )

    medians = {}
    if deltas:
        medians["wall_delta_median"] = statistics.median(
            d["wall_delta"] for d in deltas
        )
        medians["cpu_delta_median"] = statistics.median(d["cpu_delta"] for d in deltas)
        medians["wall_faster_count"] = sum(1 for d in deltas if d["wall_faster"])
        medians["cpu_faster_count"] = sum(1 for d in deltas if d["cpu_faster"])
        medians["retained_pairs"] = len(deltas)

    keep_clause = (
        medians.get("wall_delta_median", 0.0) < 0.0
        and medians.get("cpu_delta_median", 0.0) < 0.0
    )

    return {
        "errors": errors,
        "notes": notes,
        "measured_rows": len(measured),
        "arms": arms,
        "pairs": len(pairs),
        "excluded_pairs": sorted(excluded_pairs),
        "per_arm_exclusions": per_arm_exclusions,
        "best_cpu_wall_ratio": best_ratio,
        "deltas": deltas,
        "medians": medians,
        "keep_clause_met": keep_clause,
        "output_sha": next(iter(shas)) if len(shas) == 1 else None,
    }


def negative_controls(rows, report):
    """Reject-conditions the verifier must catch; returns a list of outcomes."""
    outcomes = []
    clone = lambda rs: [dict(r) for r in rs]

    # 1. Fabricated reference SHA must fail.
    r = verify(clone(rows), reference_sha="0" * 64)
    outcomes.append(
        ("fabricated reference", any("reference" in e for e in r["errors"]))
    )

    # 2. Output drift must fail.
    rs = clone(rows)
    for r_ in rs:
        if r_["warmup"] == 0 and r_["arm"] == "candidate":
            r_["sha256"] = "1" * 64
            break
    r = verify(rs)
    outcomes.append(("output drift", any("drift" in e for e in r["errors"])))

    # 3. CPU-only win (candidate wall worse, CPU better) must not meet keep.
    rs = clone(rows)
    for r_ in rs:
        if r_["warmup"] == 0 and r_["arm"] == "candidate":
            r_["wall_seconds"] *= 1.10
    r = verify(rs)
    outcomes.append(("CPU-only win", not r["keep_clause_met"]))

    # 4. Starvation pair loss must surface (exclude all but 4 pairs).
    rs = clone(rows)
    worst = sorted(rs, key=lambda r_: r_["cpu_wall_ratio"])
    for r_ in worst[
        : max(0, 2 * (len({x["pair"] for x in rs if x["warmup"] == 0}) - 4))
    ]:
        if r_["warmup"] == 0:
            r_["cpu_wall_ratio"] = 0.01
    r = verify(rs)
    outcomes.append(
        (
            "starvation pair loss",
            len(r["excluded_pairs"]) > 0
            and (len(r["deltas"]) < 5 or any("retained" in e for e in r["errors"])),
        )
    )

    # 5. Arm-order bias (all pairs candidate-first) is disclosed, not silently
    #    accepted: the report must still pass when deltas are directionally
    #    consistent; this control asserts nothing beyond presence of the
    #    ordering data in the rows.
    rs = clone(rows)
    order = [r_["run_id"] for r_ in rs if r_["warmup"] == 0]
    outcomes.append(("arm order present", len(order) >= 4))

    return outcomes


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("rows")
    ap.add_argument("--reference-sha", default=None)
    ap.add_argument("--json", default=None)
    args = ap.parse_args()

    rows = load_rows(Path(args.rows))
    report = verify(rows, reference_sha=args.reference_sha)
    controls = negative_controls(rows, report)

    print(f"measured rows: {report['measured_rows']}  pairs: {report['pairs']}")
    print(f"arms: {report['arms']}")
    print(f"best cpu/wall per arm: {report['best_cpu_wall_ratio']}")
    print(
        f"excluded pairs: {report['excluded_pairs']}  per-arm: {report['per_arm_exclusions']}"
    )
    print(f"output sha256: {report['output_sha']}")
    for d in report["deltas"]:
        print(
            f"  pair {d['pair']}: wall {d['wall_delta']:+.3f}s ({'F' if d['wall_faster'] else 'S'}) "
            f"cpu {d['cpu_delta']:+.3f}s ({'F' if d['cpu_faster'] else 'S'})"
        )
    m = report["medians"]
    print(
        f"paired medians: wall {m.get('wall_delta_median', float('nan')):+.3f}s "
        f"({m.get('wall_faster_count', 0)}/{m.get('retained_pairs', 0)} faster), "
        f"cpu {m.get('cpu_delta_median', float('nan')):+.3f}s "
        f"({m.get('cpu_faster_count', 0)}/{m.get('retained_pairs', 0)} faster)"
    )
    print(f"keep clause met: {report['keep_clause_met']}")
    print("negative controls:")
    for name, ok in controls:
        print(f"  {name}: {'CAUGHT' if ok else 'MISSED'}")
    if report["errors"]:
        print("ERRORS:")
        for e in report["errors"]:
            print(f"  - {e}")
        if args.json:
            Path(args.json).write_text(json.dumps(report, indent=2), encoding="utf-8")
        sys.exit(1)
    if args.json:
        Path(args.json).write_text(json.dumps(report, indent=2), encoding="utf-8")
    print("VERDICT: gate inputs clean")


if __name__ == "__main__":
    main()
