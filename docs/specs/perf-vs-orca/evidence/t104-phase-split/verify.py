#!/usr/bin/env python3
"""t104 phase-split take -- verifier.

Re-derives every headline number in FINDINGS.md from the committed inputs
(independent implementation of the arithmetic, no import of arithmetic.py) and
asserts structural invariants. Exits 0 only if every claim holds.

  python verify.py            # verify the committed set
  python verify.py --self-test  # negative controls: mutate the set, demand rejection

Inputs (all in-repo):
  phase-split.json      reduced instrumented phase/stage shares
  walls.csv             today's uninstrumented corroboration runs (loaded)
  counterfactuals.json  the committed derived table
  ticket 103 summary.json + t91 references.json
"""

from __future__ import annotations

import copy
import csv
import json
import statistics
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T103 = REPO / "docs/specs/perf-vs-orca/evidence/t103-orca-rebaseline/summary.json"
T91 = REPO / "docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/attribution.json"
REFS = REPO / "docs/specs/perf-vs-orca/evidence/t91-exact-z-memo/references.json"

TOL_SHARE = 0.005  # phase shares must reconcile to 1 within half a point
TOL_RATIO = 0.02  # counterfactual ratios recompute within 0.02x


def load() -> dict:
    return {
        "split": json.loads((HERE / "phase-split.json").read_text(encoding="utf-8")),
        "cf": json.loads((HERE / "counterfactuals.json").read_text(encoding="utf-8")),
        "t103": json.loads(T103.read_text(encoding="utf-8")),
        "t91": json.loads(T91.read_text(encoding="utf-8")),
        "refs": json.loads(REFS.read_text(encoding="utf-8")),
        "walls": list(csv.DictReader((HERE / "walls.csv").open(encoding="utf-8"))),
    }


def walls_medians(rows: list[dict]) -> dict[str, float]:
    by: dict[str, list[float]] = {}
    for r in rows:
        if r["warmup"] == "true" or r["exit_code"] != "0":
            continue
        by.setdefault(r["cell"], []).append(float(r["wall_s"]))
    return {k: statistics.median(v) for k, v in by.items()}


def check(bundle: dict) -> list[tuple[str, bool, str]]:
    """Return (claim, ok, detail) rows. Pure function over the bundle."""
    out: list[tuple[str, bool, str]] = []
    split, cf, t103, refs = (
        bundle["split"],
        bundle["cf"],
        bundle["t103"],
        bundle["refs"],
    )
    walls = walls_medians(bundle["walls"])

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    add(
        "every cell present in all four inputs",
        set(cf["cells"]) == set(split["cells"]) == set(walls),
        f"cf={sorted(cf['cells'])} split={sorted(split['cells'])} walls={sorted(walls)}",
    )

    for cell, c in sorted(cf["cells"].items()):
        s = split["cells"][cell]
        key = f"{cell.split('-', 1)[0]}/{cell.split('-', 1)[1]}"
        entry = t103["cells"][key]
        orca = entry["tools"]["orca"]["wall_s"]
        pnp = entry["tools"]["pnp-ordinary"]["wall_s"]

        # C1: the committed ratio is ticket 103's paired ratio.
        add(
            f"{cell}: ratio_of_record == t103 paired ratio",
            abs(c["ratio_of_record"] - pnp / orca) < TOL_RATIO,
            f"{c['ratio_of_record']} vs {pnp / orca:.2f}",
        )

        # C2: phase shares reconcile to 1.
        shares = c["phase_shares"]
        add(
            f"{cell}: phase shares reconcile to 1",
            abs(sum(shares.values()) - 1.0) <= TOL_SHARE,
            f"sum={sum(shares.values()):.4f}",
        )

        # C3: prepass stage brackets reconcile with the prepass phase.
        phase_prepass = (s["phases_ms"].get("prepass") or 0) / s["slice_complete_ms"]
        stage_prepass = sum(s["prepass_stages_ms"].values()) / s["slice_complete_ms"]
        add(
            f"{cell}: prepass stage brackets reconcile with the phase",
            abs(phase_prepass - stage_prepass) <= TOL_SHARE,
            f"phase={phase_prepass:.4f} stages={stage_prepass:.4f}",
        )

        # C4: required cut reproduces ticket 103's headline range.
        cut = 100.0 * (pnp - cf["park_ratio"] * orca) / pnp
        add(
            f"{cell}: required cut within the t103 57-82 % band",
            abs(cut - c["required_cut_pct"]) < 0.05
            and 57.0 <= c["required_cut_pct"] <= 82.0,
            f"{c['required_cut_pct']} %",
        )

        # C5: every counterfactual is (1 - share) x ratio.
        for phase in ("prepass", "per_layer", "postpass", "outside_phases"):
            want = (1.0 - shares[phase]) * c["ratio_of_record"]
            add(
                f"{cell}: ratio_if_{phase}_zero == (1-share) x ratio",
                abs(c[f"ratio_if_{phase}_zero"] - want) <= TOL_RATIO,
                f"{c[f'ratio_if_{phase}_zero']} vs {want:.2f}",
            )

        # C6: the central arithmetic. The prepass is the only phase large enough
        # to reach the park; deleting the whole per-layer phase never is; and the
        # park requires removing 70-100 % of the prepass.
        park = cf["park_ratio"]
        add(
            f"{cell}: deleting the whole per-layer phase misses the park",
            c["ratio_if_per_layer_zero"] > park,
            f"{c['ratio_if_per_layer_zero']}x > {park}",
        )
        add(
            f"{cell}: deleting both prepass dominators misses the park",
            c["ratio_if_both_dominators_zero"] > park,
            f"{c['ratio_if_both_dominators_zero']}x",
        )
        add(
            f"{cell}: deleting the whole prepass reaches the park",
            c["ratio_if_prepass_zero"] < park,
            f"{c['ratio_if_prepass_zero']}x < {park}",
        )
        frac_needed = (
            c["required_cut_pct"] / (100.0 * shares["prepass"])
            if shares["prepass"] > 0
            else float("inf")
        )
        add(
            f"{cell}: the park requires >= 70 % of the prepass removed",
            frac_needed >= 0.70,
            f"{100 * frac_needed:.1f} % of the prepass",
        )

        # C7: the per-layer pipeline is a small share and, where claimed,
        # no larger than Orca's whole slice.
        add(
            f"{cell}: per-layer phase under 25 % of the cell",
            shares["per_layer"] < 0.25,
            f"{100 * shares['per_layer']:.1f} %",
        )
        add(
            f"{cell}: per-layer implied wall <= Orca whole wall",
            c["phase_implied_wall_s"]["per_layer"] <= orca,
            f"{c['phase_implied_wall_s']['per_layer']}s vs orca {orca:.2f}s",
        )

        # C8: exactness and cleanliness of the instrumented pass.
        ref = refs["cells"].get(f"ordinary-{cell}", {}).get("sha256")
        add(
            f"{cell}: instrumented output is the t87 reference",
            ref is not None and s["gcode_sha256"] == ref,
            f"{s['gcode_sha256'][:12]} vs {(ref or 'missing')[:12]}",
        )
        add(
            f"{cell}: instrumented run clean",
            s["degraded"] is False and s["non_fatal_error_count"] == 0,
            f"degraded={s['degraded']} non_fatal={s['non_fatal_error_count']}",
        )

        # C9: the disclosed load inflation is positive (walls are corroboration).
        add(
            f"{cell}: loaded corroboration wall exceeds the quiet record",
            c["load_inflation"] is not None and c["load_inflation"] > 1.0,
            f"x{c['load_inflation']}",
        )

    # C10: the split's semantics are unchanged since the 2026-10-27 vintage.
    # t91's baseline capture of ordinary base classic supports-on recorded
    # prepass_phase_ms 149,723 of slice_ms 168,319.
    b = bundle["t91"]["cells"]["ordinary-base-classic-on"]["baseline"]
    t91_share = b["prepass_phase_ms"] / b["slice_ms"]
    ours = cf["cells"]["base-classic-on"]["phase_shares"]["prepass"]
    add(
        "prepass share matches the 2026-10-27 vintage (t91 baseline)",
        abs(ours - t91_share) < 0.02,
        f"{100 * ours:.1f} % vs {100 * t91_share:.1f} %",
    )
    return out


def run(bundle: dict, label: str) -> bool:
    rows = check(bundle)
    bad = [r for r in rows if not r[1]]
    print(f"== {label}: {len(rows) - len(bad)}/{len(rows)} claims hold")
    for name, ok, detail in rows:
        if not ok:
            print(f"   FAIL {name} ({detail})")
    return not bad


def main() -> int:
    bundle = load()
    ok = run(bundle, "committed set")

    if "--self-test" in sys.argv:
        controls: list[tuple[str, callable]] = []

        def m_zero_share(b: dict) -> None:
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["phase_shares"]["prepass"] = 0.0

        def m_swap_sha(b: dict) -> None:
            cells = sorted(b["split"]["cells"])
            a, c = cells[0], cells[-1]
            (
                b["split"]["cells"][a]["gcode_sha256"],
                b["split"]["cells"][c]["gcode_sha256"],
            ) = (
                b["split"]["cells"][c]["gcode_sha256"],
                b["split"]["cells"][a]["gcode_sha256"],
            )

        def m_drop_stage(b: dict) -> None:
            first = next(iter(b["split"]["cells"]))
            stages = b["split"]["cells"][first]["prepass_stages_ms"]
            stages.pop(next(iter(stages)))

        def m_fake_cut(b: dict) -> None:
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["required_cut_pct"] = 10.0

        def m_fake_ratio(b: dict) -> None:
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["ratio_if_prepass_zero"] = 1.05

        def m_fake_ratio_of_record(b: dict) -> None:
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["ratio_of_record"] = 1.0

        def m_dirty(b: dict) -> None:
            first = next(iter(b["split"]["cells"]))
            b["split"]["cells"][first]["non_fatal_error_count"] = 3

        def m_negative_inflation(b: dict) -> None:
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["load_inflation"] = 0.5

        def m_missing_cell(b: dict) -> None:
            b["walls"] = [
                r for r in b["walls"] if r["cell"] != next(iter(b["cf"]["cells"]))
            ]

        controls = [
            ("zeroed prepass share", m_zero_share),
            ("swapped output SHA", m_swap_sha),
            ("dropped prepass stage", m_drop_stage),
            ("faked required cut", m_fake_cut),
            ("faked counterfactual ratio", m_fake_ratio),
            ("faked ratio of record", m_fake_ratio_of_record),
            ("dirty instrumented run", m_dirty),
            ("non-positive load inflation", m_negative_inflation),
            ("cell missing from walls", m_missing_cell),
        ]
        detected = 0
        for name, mutate in controls:
            b = copy.deepcopy(bundle)
            mutate(b)
            rows = check(b)
            caught = any(not ok for _, ok, _ in rows)
            print(f"   control [{name}]: {'detected' if caught else 'MISSED'}")
            detected += 1 if caught else 0
        print(f"== self-test: {detected}/{len(controls)} negative controls detected")
        ok = ok and detected == len(controls)

    print("PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
