#!/usr/bin/env python3
"""t110 verifier: re-derive the phase tables from runs.csv and check the invariants.

Runs checkout-only (no raw run tree, no Orca, no cargo). Exits 0 on success, 1 on a
failed check. `--self-test` mutates the committed artifacts in memory and requires
every negative control to be rejected.

Checks, in order:
  1. every cell has the pre-declared number of retained runs (3) and no failed run;
  2. the tap's boundaries are ordered 0 < t_tmp <= t_rename <= wall, and CPU is
     monotone across the same boundaries;
  3. every repeat of a cell emits the same G-code byte count (deterministic job);
  4. shares sum to 100 % and the shares/sum/medians match phase-split.json;
  5. PnP ratios are reproduced independently from t104's phase-split.json;
  6. the info controls have two runs per model and a spread under 15 %.
"""

import csv
import json
import os
import statistics
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
RUNS = os.path.join(HERE, "runs.csv")
SPLIT = os.path.join(HERE, "phase-split.json")
T104 = os.path.join(HERE, "..", "t104-phase-split", "phase-split.json")
CELLS = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]
MODEL_OF = {c: ("benchy" if c.startswith("benchy") else "base") for c in CELLS}
REPEATS = 3


class Check:
    def __init__(self):
        self.failures = []

    def ok(self, cond, msg):
        if not cond:
            self.failures.append(msg)
        return cond

    def close(self, a, b, tol, msg):
        return self.ok(
            a is not None and b is not None and abs(a - b) <= tol,
            f"{msg}: {a} vs {b} (tol {tol})",
        )


def fnum(row, key):
    v = row.get(key)
    return None if v in (None, "") else float(v)


def verify(rows, split, t104, check):
    info = {}
    for model in ("calicat", "benchy", "base"):
        vals = [
            fnum(r, "wall_ms")
            for r in rows
            if r["kind"] == "info" and r["model"] == model
        ]
        check.ok(len(vals) == 2, f"info {model}: expected 2 runs, got {len(vals)}")
        if len(vals) >= 2:
            info[model] = statistics.median(vals)
            spread = 100.0 * (max(vals) - min(vals)) / info[model]
            check.ok(spread < 15.0, f"info {model}: spread {spread:.1f}% over 15%")
    check.ok(
        (split["info_controls"]["base"]["wall_ms_median"] - info.get("base", -1))
        < 0.06,
        "phase-split base info median disagrees with runs.csv",
    )

    for cell in CELLS:
        rs = [r for r in rows if r["kind"] == "slice" and r["cell"] == cell]
        c = check.ok(
            len(rs) == REPEATS, f"{cell}: expected {REPEATS} runs, got {len(rs)}"
        )
        if not c:
            continue
        for r in rs:
            check.ok(
                int(r["exit_code"]) == 0, f"{cell}/{r['run_id']}: exit {r['exit_code']}"
            )
            t_tmp, t_ren, wall = (
                fnum(r, "t_tmp_ms"),
                fnum(r, "t_rename_ms"),
                fnum(r, "wall_ms"),
            )
            check.ok(
                t_tmp is not None and 0 < t_tmp <= t_ren <= wall,
                f"{cell}/{r['run_id']}: boundary order t_tmp={t_tmp} t_rename={t_ren} wall={wall}",
            )
            cpu_t, cpu_r, cpu_f = (
                fnum(r, "cpu_at_tmp_s"),
                fnum(r, "cpu_at_rename_s"),
                fnum(r, "cpu_final_s"),
            )
            check.ok(
                cpu_t is not None and cpu_t <= cpu_r <= cpu_f,
                f"{cell}/{r['run_id']}: CPU not monotone {cpu_t} {cpu_r} {cpu_f}",
            )
        bytes_ = {int(fnum(r, "gcode_bytes")) for r in rs}
        check.ok(
            len(bytes_) == 1,
            f"{cell}: repeats disagree on G-code bytes: {sorted(bytes_)}",
        )
        log_bytes = [int(fnum(r, "log_bytes")) for r in rs]
        check.ok(
            min(log_bytes) > 100_000,
            f"{cell}: a run's CLI log is suspiciously small {log_bytes}",
        )

        walls = [fnum(r, "wall_ms") for r in rs]
        pres = [fnum(r, "prephase_ms") for r in rs]
        emits = [fnum(r, "emit_ms") for r in rs]
        wall_m, pre_m, emit_m = (
            statistics.median(walls),
            statistics.median(pres),
            statistics.median(emits),
        )
        prepass = pre_m - info[MODEL_OF[cell]]

        node = split["cells"][cell]
        o = node["orca"]
        check.close(o["wall_ms_median"], wall_m, 0.5, f"{cell}: wall median")
        check.close(o["prephase_ms_median"], pre_m, 0.5, f"{cell}: prephase median")
        check.close(o["emit_ms_median"], emit_m, 0.5, f"{cell}: emit median")
        check.close(o["prepass_ms"], prepass, 0.6, f"{cell}: derived prepass")
        check.close(
            o["startup_import_ms"], info[MODEL_OF[cell]], 0.6, f"{cell}: startup+import"
        )

        shares = o["shares_pct"]
        total_share = sum(shares.values())
        # Each share is rounded to 0.01 pp independently and the phases come from
        # separate medians, so the sum carries up to ~0.25 pp of rounding drift.
        check.ok(
            abs(total_share - 100.0) <= 0.5, f"{cell}: shares sum to {total_share:.2f}%"
        )
        check.close(
            shares["prepass"], 100.0 * prepass / wall_m, 0.02, f"{cell}: prepass share"
        )
        check.close(
            shares["emit"], 100.0 * emit_m / wall_m, 0.02, f"{cell}: emit share"
        )

        pnp = t104["cells"][cell]
        pnp_prepass = pnp["phases_ms"]["prepass"]
        check.ok(
            abs(node["pnp_t104"]["prepass_ms"] - pnp_prepass) <= 1,
            f"{cell}: PnP prepass drifted from t104 ({node['pnp_t104']['prepass_ms']} vs {pnp_prepass})",
        )
        check.close(
            node["compare"]["prepass_multiple_pnp_over_orca"],
            pnp_prepass / prepass,
            0.02,
            f"{cell}: prepass multiple",
        )
        pnp_emit = pnp["phases_ms"]["per_layer"] + pnp["phases_ms"]["postpass"]
        check.close(
            node["compare"]["emit_multiple_pnp_over_orca"],
            pnp_emit / emit_m,
            0.02,
            f"{cell}: emit multiple",
        )
        check.close(
            node["pnp_t104"]["prepass_share_pct"],
            100.0 * pnp_prepass / pnp["slice_complete_ms"],
            0.02,
            f"{cell}: PnP prepass share",
        )

    # The tap is only allowed to claim a point when the phase is >= 3 poll intervals.
    worst_gap = max(float(r["poll_max_gap_ms"]) for r in rows if r["kind"] == "slice")
    smallest_phase = min(split["cells"][c]["orca"]["emit_ms_median"] for c in CELLS)
    check.ok(
        smallest_phase >= 3 * worst_gap,
        f"smallest phase {smallest_phase} ms is under 3 poll gaps ({worst_gap} ms)",
    )


def load():
    with open(RUNS, newline="", encoding="utf-8") as fh:
        rows = list(csv.DictReader(fh))
    split = json.load(open(SPLIT, encoding="utf-8"))
    t104 = json.load(open(T104, encoding="utf-8"))
    return rows, split, t104


def self_test():
    rows, split, t104 = load()
    controls = []

    # 1. wall inflation on a measured slice row
    r1 = [dict(r) for r in rows]
    for r in r1:
        if r["kind"] == "slice" and r["cell"] == CELLS[2]:
            r["wall_ms"] = str(float(r["wall_ms"]) * 1.10)
            break
    controls.append(("wall +10%", r1, split, t104))

    # 2. emit shortened on one run
    r2 = [dict(r) for r in rows]
    for r in r2:
        if r["kind"] == "slice" and r["cell"] == CELLS[0]:
            r["emit_ms"] = str(float(r["emit_ms"]) * 0.85)
            break
    controls.append(("emit -15%", r2, split, t104))

    # 3. info control moved
    r3 = [dict(r) for r in rows]
    for r in r3:
        if r["kind"] == "info" and r["model"] == "base":
            r["wall_ms"] = str(float(r["wall_ms"]) + 400.0)
    controls.append(("info +400 ms", r3, split, t104))

    # 4. a slice row dropped
    r4 = [
        r
        for r in rows
        if not (
            r["kind"] == "slice"
            and r["cell"] == CELLS[1]
            and r["run_id"].endswith("m3")
        )
    ]
    controls.append(("dropped repeat", r4, split, t104))

    # 5. G-code bytes made non-deterministic
    r5 = [dict(r) for r in rows]
    for r in r5:
        if r["kind"] == "slice" and r["cell"] == CELLS[2]:
            r["gcode_bytes"] = str(int(float(r["gcode_bytes"])) + 1)
            break
    controls.append(("bytes differ", r5, split, t104))

    # 6. claimed multiple tampered
    s6 = json.loads(json.dumps(split))
    s6["cells"][CELLS[3]]["compare"]["prepass_multiple_pnp_over_orca"] *= 0.8
    controls.append(("multiple tampered", rows, s6, t104))

    # 7. t104 PnP side shifted
    t7 = json.loads(json.dumps(t104))
    t7["cells"][CELLS[0]]["phases_ms"]["prepass"] = int(
        t7["cells"][CELLS[0]]["phases_ms"]["prepass"] * 0.5
    )
    controls.append(("t104 prepass halved", rows, split, t7))

    rejected = 0
    for name, r, s, t in controls:
        c = Check()
        try:
            verify(r, s, t, c)
        except Exception:
            c.failures.append("raised")
        if c.failures:
            rejected += 1
            print(f"  rejected: {name}")
        else:
            print(f"  NOT REJECTED: {name}")
    print(f"\nnegative controls rejected: {rejected}/{len(controls)}")
    return 0 if rejected == len(controls) else 1


def main():
    if "--self-test" in sys.argv:
        sys.exit(self_test())
    rows, split, t104 = load()
    check = Check()
    verify(rows, split, t104, check)
    if check.failures:
        print(f"FAIL: {len(check.failures)} check(s) failed")
        for f in check.failures:
            print("  -", f)
        sys.exit(1)
    print(
        f"PASS: {len(CELLS)} cells re-derived from {len(rows)} rows; "
        f"{REPEATS} repeats per cell; shares sum to 100%; PnP multiples reproduced from t104"
    )


if __name__ == "__main__":
    main()
