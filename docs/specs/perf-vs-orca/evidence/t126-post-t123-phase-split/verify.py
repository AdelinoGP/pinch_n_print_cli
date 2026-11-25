#!/usr/bin/env python3
"""t126 -- verifier.

Re-derives every headline number in FINDINGS.md from the committed inputs
(independent implementation; no import of arithmetic.py or reduce.py) and
asserts structural invariants. Exits 0 only if every claim holds.

  python verify.py              # verify the committed set
  python verify.py --self-test  # negative controls: mutate the set, demand rejection

Inputs (all in-repo):
  summary.json                                 reduced instrumented shares (this take)
  counterfactuals.json                         the committed derived table
  ../t123-facet-retirement/reduced/summary.json  output reference set
  ../t124-post-t123-scoreboard/scoreboard-summary.json  paired medians of record
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVID = HERE.parent
T123 = EVID / "t123-facet-retirement" / "reduced" / "summary.json"
T124 = EVID / "t124-post-t123-scoreboard" / "scoreboard-summary.json"

TOL_SHARE = 0.005  # shares reconcile to 1 within half a point
TOL_RATIO = 0.02  # recomputed ratios within 0.02x
TOL_MS = 1.0  # ms arithmetic within 1 ms


def load() -> dict:
    return {
        "summary": json.loads((HERE / "summary.json").read_text(encoding="utf-8")),
        "cf": json.loads((HERE / "counterfactuals.json").read_text(encoding="utf-8")),
        "t123": json.loads(T123.read_text(encoding="utf-8")),
        "t124": json.loads(T124.read_text(encoding="utf-8")),
    }


def key(name: str) -> str:
    fixture, generator, supports = name.split("-", 2)
    return f"{fixture}/{generator}/{supports}"


def check(bundle: dict) -> list[tuple[str, bool, str]]:
    out: list[tuple[str, bool, str]] = []
    summary = bundle["summary"]["cells"]
    cf = bundle["cf"]["cells"]
    record = bundle["t124"]["cells"]
    refs = bundle["t123"]["output"]

    def add(name: str, ok: bool, detail: str = "") -> None:
        out.append((name, bool(ok), detail))

    add(
        "same eight cells in summary, counterfactuals and both upstream sets",
        len(summary) == 8
        and set(summary) == set(cf)
        and {key(c) for c in summary} == set(record)
        and set(summary) == set(refs),
        f"{sorted(summary)}",
    )

    for name, s in sorted(summary.items()):
        c = cf[name]
        rec = record[key(name)]
        pnp = rec["pnp-ordinary"]["wall_s"]
        orca = rec["orca"]["wall_s"]

        # C1: ratio of record is ticket 124's paired ordinary median ratio.
        add(
            f"{name}: ratio_of_record == t124 paired ratio",
            abs(c["ratio_of_record"] - pnp / orca) < TOL_RATIO
            and abs(c["ratio_of_record"] - rec["ordinary_ratio"]) < TOL_RATIO,
            f"{c['ratio_of_record']:.4f} vs {pnp / orca:.4f}",
        )

        # C2: phase shares reconcile to 1.
        total = sum(c["share"].values())
        add(
            f"{name}: phase shares reconcile to 1",
            abs(total - 1.0) <= TOL_SHARE,
            f"sum={total:.4f}",
        )

        # C3: each share and implied wall recompute from the reduced stream.
        complete = s["slice_complete_ms"]
        want = {p: (ms or 0) / complete for p, ms in s["phases_ms"].items()}
        want["outside"] = s["outside_phases_ms"] / complete
        add(
            f"{name}: shares recompute from the stream",
            all(abs(c["share"][p] - want[p]) <= 1e-6 for p in want),
            f"{want}",
        )
        add(
            f"{name}: implied walls recompute from the t124 median",
            all(abs(c["implied_wall_s"][p] - want[p] * pnp) <= 1e-2 for p in want),
            f"pnp={pnp}",
        )

        # C4: phase brackets and the outside residual reconcile with slice_complete.
        inner = sum(v or 0 for v in s["phases_ms"].values())
        add(
            f"{name}: phases + outside == slice_complete",
            abs(inner + s["outside_phases_ms"] - complete) <= TOL_MS,
            f"{inner}+{s['outside_phases_ms']} vs {complete}",
        )

        # C5: prepass stage brackets reconcile with the prepass phase.
        stage_sum = sum(s["prepass_stages_ms"].values())
        add(
            f"{name}: prepass stages reconcile with the prepass phase",
            abs(stage_sum - (s["phases_ms"].get("prepass") or 0))
            <= 0.02 * (s["phases_ms"].get("prepass") or 0),
            f"{stage_sum} vs {s['phases_ms'].get('prepass')}",
        )

        # C6: per-layer work is parallel (thread sum > wall) and the phase is
        # saturated: the module-busy union covers the wall minus the host gap.
        detail = s["phase_detail"]["per_layer"]
        add(
            f"{name}: per-layer thread sum exceeds its wall (parallel phase)",
            sum(s["per_layer_stages_thread_sum_ms"].values()) > detail["wall_ms"] > 0,
            f"thread={sum(s['per_layer_stages_thread_sum_ms'].values())} "
            f"wall={detail['wall_ms']}",
        )
        add(
            f"{name}: per-layer busy union + host gap == phase wall",
            abs(
                detail["module_busy_union_ms"]
                + detail["host_gap_ms"]
                - detail["wall_ms"]
            )
            <= TOL_MS
            and detail["host_gap_ms"] >= 0,
            f"{detail}",
        )

        # C7: counterfactuals are (1 - share) x ratio.
        for phases, field in (
            (("prepass",), "ratio_if_prepass_zero"),
            (("per_layer",), "ratio_if_per_layer_zero"),
            (("prepass", "per_layer"), "ratio_if_prepass_and_per_layer_zero"),
        ):
            expect = c["ratio_of_record"] * (1.0 - sum(c["share"][p] for p in phases))
            add(
                f"{name}: {field} == (1 - share) x ratio",
                abs(c[field] - expect) <= TOL_RATIO,
                f"{c[field]:.4f} vs {expect:.4f}",
            )

        # C8: cuts reproduce from the ratio.
        add(
            f"{name}: cut_to_beat reproduces",
            abs(c["cut_to_beat"] - (1.0 - 1.0 / c["ratio_of_record"])) < 1e-6,
            f"{c['cut_to_beat']:.4f}",
        )
        add(
            f"{name}: floor is 0.80 % of the t124 median",
            abs(c["floor_ms"] - 0.008 * pnp * 1000.0) <= 0.1,
            f"{c['floor_ms']} ms",
        )

        # C9: output identity and cleanliness of the instrumented pass.
        ref = refs[name]["candidate-ordinary"]["sha256"]
        add(
            f"{name}: instrumented output is the t123 reference",
            s["gcode_sha256"] == ref,
            f"{s['gcode_sha256'][:12]} vs {ref[:12]}",
        )
        add(
            f"{name}: instrumented run clean",
            s["degraded"] is False
            and s["fatal_error_count"] == 0
            and s["non_fatal_error_count"] == 0,
            f"degraded={s['degraded']} fatal={s['fatal_error_count']} "
            f"non_fatal={s['non_fatal_error_count']}",
        )

        # C10: module call statistics integrate to the module thread sums.
        for phase, stats in s["module_call_stats"].items():
            for mid, st in stats.items():
                thread = s["module_thread_sum_ms"][phase][mid]
                add(
                    f"{name}: {phase}/{mid} call stats integrate",
                    st["calls"] == s["module_calls"][phase][mid]
                    and st["thread_sum_ms"] == thread
                    and st["p50_ms"] <= st["max_ms"],
                    f"calls={st['calls']} sum={st['thread_sum_ms']} vs {thread}",
                )

        # C11: the wall-equivalent ceiling is thread sum / measured parallelism.
        par = c["per_layer_parallelism"]
        add(
            f"{name}: per-layer parallelism reproduces",
            abs(
                par
                - sum(s["per_layer_stages_thread_sum_ms"].values())
                / s["phases_ms"]["per_layer"]
            )
            < 1e-3,
            f"{par}",
        )
        for mid, mod in c["top_per_layer_modules"].items():
            expect = mod["thread_sum_ms"] / par
            add(
                f"{name}: {mid} ceiling == thread/parallelism",
                abs(mod["wall_equivalent_ceiling_ms"] - expect) <= TOL_MS,
                f"{mod['wall_equivalent_ceiling_ms']} vs {expect:.1f}",
            )
            add(
                f"{name}: {mid} ceiling multiple == ceiling/floor",
                abs(mod["floor_multiple_ceiling"] - expect / c["floor_ms"]) < 0.02,
                f"{mod['floor_multiple_ceiling']}",
            )

    # C12: the headline arithmetic -- the prepass still dominates and the
    # per-layer phase alone cannot close any cell.
    for name, c in sorted(cf.items()):
        add(
            f"{name}: prepass is the largest phase",
            c["share"]["prepass"]
            >= max(
                c["share"]["per_layer"], c["share"]["postpass"], c["share"]["outside"]
            ),
            f"prepass {c['share']['prepass']:.3f}",
        )
        add(
            f"{name}: deleting the per-layer phase alone does not beat Orca",
            c["ratio_if_per_layer_zero"] > 1.0,
            f"{c['ratio_if_per_layer_zero']:.2f}x",
        )
        add(
            f"{name}: the per-layer phase is under 40 % of the cell",
            c["share"]["per_layer"] < 0.40,
            f"{100 * c['share']['per_layer']:.1f} %",
        )

    # C13: the named per-layer anomaly -- raft-default pays a dispatch cost on
    # every base layer while its body early-outs (raft_layers == 2), and is
    # negligible on the benchy fixture.
    for name, s in sorted(summary.items()):
        st = s["module_call_stats"]["per_layer"].get("com.core.raft-default")
        if st is None:
            add(f"{name}: raft-default present", False, "missing")
            continue
        if name.startswith("base-"):
            add(
                f"{name}: raft-default pays on every layer",
                st["calls"] == 495
                and st["calls_gt_10ms"] == st["calls"]
                and st["p50_ms"] >= 60,
                f"{st['calls_gt_10ms']}/{st['calls']} calls > 10 ms, "
                f"p50={st['p50_ms']} ms",
            )
        else:
            add(
                f"{name}: raft-default is negligible on benchy",
                st["p50_ms"] <= 10 and st["calls_gt_10ms"] <= 0.10 * st["calls"],
                f"p50={st['p50_ms']} ms, "
                f"{st['calls_gt_10ms']}/{st['calls']} calls > 10 ms",
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
        controls = []

        def control(name, mutate):
            controls.append((name, mutate))

        def m_zero_share(b):
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["share"]["prepass"] = 0.0

        def m_swap_sha(b):
            cells = sorted(b["summary"]["cells"])
            a, c = cells[0], cells[-1]
            (
                b["summary"]["cells"][a]["gcode_sha256"],
                b["summary"]["cells"][c]["gcode_sha256"],
            ) = (
                b["summary"]["cells"][c]["gcode_sha256"],
                b["summary"]["cells"][a]["gcode_sha256"],
            )

        def m_fake_ratio(b):
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["ratio_if_prepass_zero"] = 1.05

        def m_fake_record(b):
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["ratio_of_record"] = 1.0

        def m_dirty(b):
            first = next(iter(b["summary"]["cells"]))
            b["summary"]["cells"][first]["non_fatal_error_count"] = 3

        def m_drop_module(b):
            first = next(iter(b["summary"]["cells"]))
            b["summary"]["cells"][first]["module_call_stats"]["per_layer"].pop(
                "com.core.raft-default"
            )

        def m_fake_parallelism(b):
            first = next(iter(b["cf"]["cells"]))
            b["cf"]["cells"][first]["per_layer_parallelism"] = 1.0

        def m_fake_host_gap(b):
            first = next(iter(b["summary"]["cells"]))
            b["summary"]["cells"][first]["phase_detail"]["per_layer"]["host_gap_ms"] = (
                99
            )

        def m_fake_raft_calls(b):
            first = next(iter(b["summary"]["cells"]))
            b["summary"]["cells"][first]["module_call_stats"]["per_layer"][
                "com.core.raft-default"
            ]["calls_gt_10ms"] = 0

        def m_missing_cell(b):
            b["summary"]["cells"].pop(sorted(b["summary"]["cells"])[0])

        control("zeroed prepass share", m_zero_share)
        control("swapped output SHA", m_swap_sha)
        control("faked counterfactual ratio", m_fake_ratio)
        control("faked ratio of record", m_fake_record)
        control("dirty instrumented run", m_dirty)
        control("dropped raft-default module stats", m_drop_module)
        control("faked per-layer parallelism", m_fake_parallelism)
        control("faked host gap", m_fake_host_gap)
        control("raft-default cost erased", m_fake_raft_calls)
        control("cell missing from summary", m_missing_cell)

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
