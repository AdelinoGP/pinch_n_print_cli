#!/usr/bin/env python
"""Ticket 17 cost A/B: interleaved paired runs of the two clipper versions.

Design (per the t15 finding #1 and PERF-HANDOFF §3.6 / §10.3):
- ONE binary per version, built from trees that differ only in the clipper pin.
- Alternate runs A,B,A,B,... so external load hits both sides.
- Save each run under a distinct criterion baseline directory
  (`criterion-interleaved/<home>`), then read `estimates.json` per run and
  compute the per-run paired delta (median of per-run deltas).
- Report CV per run so a starved sample is visible.

Usage: python interleave.py <N_rounds> [--reverse]
"""

import glob
import json
import os
import subprocess
import sys
import statistics

BASE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(BASE))  # repo root (target/../..)

EXE_103 = os.path.join(
    ROOT, "target", "t17-clipper2-ab", "worktree-target", "release", "deps"
)
EXE_110 = os.path.join(ROOT, "target", "release", "deps")

# Env overrides let the same harness run arbitrary binary pairs (e.g. the
# same-version cross-tree control).
EXE_A = os.environ.get("T17_EXE_A")
EXE_B = os.environ.get("T17_EXE_B")
LABEL_A = os.environ.get("T17_LABEL_A", "A")
LABEL_B = os.environ.get("T17_LABEL_B", "B")
HOME_A = os.environ.get("T17_HOME", "interleaved")

# Filters: the four surfaces named in the ticket, 256-square leaves (largest,
# lowest CV) plus 64 for a second point.
FILTERS = [
    "union/256",
    "intersection/256",
    "difference/256",
    "offset/256",
    "union/64",
    "intersection/64",
    "difference/64",
    "offset/64",
]


def find_exe(d, needle):
    cands = [p for p in glob.glob(os.path.join(d, "polygon_ops-*.exe"))]
    if not cands:
        return None
    # Prefer the newest (largest mtime)
    return max(cands, key=os.path.getmtime)


def resolve_exes():
    """Return (exe_a, exe_b) for the run, honouring env overrides."""
    if EXE_A and EXE_B:
        return EXE_A, EXE_B
    return find_exe(EXE_103, "103"), find_exe(EXE_110, "110")


def read_run(home, run):
    """Read estimates for one run directory tree."""
    out = {}
    for est in glob.glob(
        os.path.join(home, "**", run, "estimates.json"), recursive=True
    ):
        parts = est.replace("\\", "/").split("/")
        leaf = parts[-4].replace("polygon_ops_", "") + "/" + parts[-3]
        with open(est) as fh:
            j = json.load(fh)
        out[leaf] = {
            "mean": j["mean"]["point_estimate"],
            "slope": j["slope"]["point_estimate"] if j.get("slope") else None,
            "cv": j["std_dev"]["point_estimate"] / j["mean"]["point_estimate"] * 100,
        }
    return out


def main():
    rounds = int(sys.argv[1]) if len(sys.argv) > 1 else 4
    reverse = "--reverse" in sys.argv
    exe103, exe110 = resolve_exes()
    print(f"A exe: {os.path.basename(exe103)} ({LABEL_A})")
    print(f"B exe: {os.path.basename(exe110)} ({LABEL_B})")
    print(f"order: {'B-then-A' if reverse else 'A-then-B'}")

    # Order control: A-then-B inflates B when load drifts downward over the
    # round (the 10-round 2026-09-24 run showed all-positive deltas for exactly
    # this reason). Running a reversed-order pass and averaging the two orders
    # cancels first-order drift.
    order = [("A", exe103), ("B", exe110)]
    if reverse:
        order = [("B", exe110), ("A", exe103)]

    pair_data = {}  # leaf -> list of (a_mean, b_mean, a_cv, b_cv)
    home = os.path.join(BASE, HOME_A + ("-rev" if reverse else ""))
    summary_name = f"{HOME_A}{'-rev' if reverse else ''}-summary.json"
    for r in range(rounds):
        for label, exe in order:
            env = dict(os.environ, CRITERION_HOME=home)
            print(f"  round {r} {label}")
            # One invocation per filter: criterion's positional arg is a
            # substring filter, not a regex, so a `|`-joined list matches nothing.
            for filt in FILTERS:
                run = f"r{r}{label}-{filt.replace('/', '_')}"
                cmd = [exe, "--bench", "--save-baseline", run, "--noplot", filt]
                log = os.path.join(BASE, f"interleaved-log-{run}.txt")
                with open(log, "wb") as fh:
                    subprocess.run(
                        cmd, env=env, check=True, stdout=fh, stderr=subprocess.STDOUT
                    )
        # read both sides of this round
        a = {}
        b = {}
        for filt in FILTERS:
            tag = filt.replace("/", "_")
            a.update(read_run(home, f"r{r}A-{tag}"))
            b.update(read_run(home, f"r{r}B-{tag}"))
        for k in a:
            if k in b:
                pair_data.setdefault(k, []).append(
                    (
                        a[k]["mean"],
                        b[k]["mean"],
                        a[k]["slope"],
                        b[k]["slope"],
                        a[k]["cv"],
                        b[k]["cv"],
                    )
                )

    print()
    print(
        f"{'leaf':20s} {'A mean us':>10s} {'B mean us':>10s} {'paired d%':>10s} {'median d%':>10s} "
        f"{'A cv':>6s} {'B cv':>6s} {'n':>3s}"
    )
    summary = {}
    for k in sorted(pair_data, key=lambda s: (s.split("/")[0], int(s.split("/")[1]))):
        rows = pair_data[k]
        deltas = [((b / a) - 1) * 100 for (a, b, *_) in rows]
        ameans = [a for (a, *_) in rows]
        bmeans = [b for (_, b, *_) in rows]
        acv = [r[4] for r in rows]
        bcv = [r[5] for r in rows]
        med = statistics.median(deltas)
        summary[k] = {
            "deltas": deltas,
            "median": med,
            "a_mean": statistics.median(ameans),
            "b_mean": statistics.median(bmeans),
        }
        print(
            f"{k:20s} {statistics.median(ameans) / 1000:10.2f} {statistics.median(bmeans) / 1000:10.2f} "
            f"{statistics.median(deltas):+9.2f}% {med:+9.2f}% {statistics.median(acv):5.2f}% "
            f"{statistics.median(bcv):5.2f}% {len(rows):3d}"
        )

    with open(
        os.path.join(BASE, summary_name),
        "w",
    ) as fh:
        json.dump(summary, fh, indent=1)
    print(f"\nwrote {summary_name}")


if __name__ == "__main__":
    main()
