#!/usr/bin/env python3
"""t165 verify: re-derive reduced.json and FINDINGS.md's claims from runs.csv.

Exit 0 when every check passes. `--self-test` mutates the inputs in memory and
requires every negative control to be rejected.
Stdlib only; reads runs.csv, reduced.json, FINDINGS.md and t164's reduced set.
"""

from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import reduce as R

HERE = Path(__file__).resolve().parent
EXE = "5013b1bdeb52a097e1fe2357178342461cf56cc5dccd58479ccd3999f997853c"


def checks(rows, board, split, committed, findings):
    errs = []
    if len(rows) != 24 or sum(r["measured"] == "1" for r in rows) != 18:
        errs.append("row count: want 24 rows / 18 measured")
    if any(r["exit_code"] != "0" for r in rows):
        errs.append("a run exited non-zero")
    if {r["exe_sha256"] for r in rows} != {EXE}:
        errs.append("executable identity is not the t161 2.5.1 boundary")
    if any(abs(float(r["wall_agree_pct"])) >= 1.0 for r in rows):
        errs.append("tap wall disagrees with GetProcessTimes wall by >= 1 %")
    if any(r["kind"] == "slice" and (not r["t_tmp_ms"] or not r["t_rename_ms"]) for r in rows):
        errs.append("a slice row lacks a tap boundary")
    got = R.reduce(rows, board, split)
    if json.dumps(got, sort_keys=True) != json.dumps(committed, sort_keys=True):
        errs.append("reduced.json does not re-derive from runs.csv")
    c = got["classes"]
    if c["benchy-off"]["label"] != "bound":
        errs.append("benchy-off must be a bound")
    if any(c[k]["label"] != "point" for k in ("benchy-on", "base-off", "base-on")):
        errs.append("supports-on / base-off must be points")
    # ticket question: benchy supports-on within 1.53x of t110
    if not c["benchy-on"]["prepass_ms"] < 1.53 * 549.8:
        errs.append("benchy-on prepass is not within 1.53x of t110")
    s = got["summary"]
    if s["cells_winning"] != 8 or s["binding_cell"] != "benchy-arachne-on":
        errs.append("projection summary changed")
    for needle in (
        f"{c['benchy-off']['prepass_ms']:.1f}", f"{c['benchy-on']['prepass_ms']:.1f}",
        f"{c['base-off']['prepass_ms']:,.1f}", f"{c['base-on']['prepass_ms']:,.1f}",
        f"{s['binding_result']:.3f}x", f"{s['binding_slack']:.2f}x",
        f"{got['max_abs_wall_agree_pct']:.3f} %",
    ):
        if needle not in findings:
            errs.append(f"FINDINGS.md does not state {needle}")
    for cell, v in got["cells"].items():
        if f"{v['result_t165']:.3f}x" not in findings:
            errs.append(f"FINDINGS.md lacks {cell} result {v['result_t165']:.3f}x")
    return errs


def load():
    rows = R.load_rows(HERE / "runs.csv")
    board = json.loads((R.T164 / "board.json").read_text(encoding="utf-8"))
    split = json.loads((R.T164 / "instrumented.json").read_text(encoding="utf-8"))
    committed = json.loads((HERE / "reduced.json").read_text(encoding="utf-8"))
    findings = (HERE / "FINDINGS.md").read_text(encoding="utf-8")
    return rows, board, split, committed, findings


def self_test() -> int:
    base = load()
    if checks(*base):
        print("baseline fails; fix that first")
        return 1

    def mut(fn):
        x = copy.deepcopy(base)
        fn(x)
        return x

    def row(x, cls, tag="m1"):
        return next(r for r in x[0] if r["cls"] == cls and r["run_id"].endswith(tag))

    controls = {
        "tmp moved": lambda x: row(x, "benchy-on").__setitem__("t_tmp_ms", "2400.0"),
        "info moved": lambda x: row(x, "info-base").__setitem__("tap_wall_ms", "8000.0"),
        "exe swapped": lambda x: row(x, "base-on").__setitem__("exe_sha256", "0" * 64),
        "wall disagree": lambda x: row(x, "base-off").__setitem__("wall_agree_pct", "1.2"),
        "nonzero exit": lambda x: row(x, "benchy-off").__setitem__("exit_code", "1"),
        "warmup counted": lambda x: next(r for r in x[0] if r["run_id"] == "base-on-w").__setitem__("measured", "1"),
        "row dropped": lambda x: x[0].pop(),
        "missing boundary": lambda x: row(x, "base-on", "m2").__setitem__("t_rename_ms", ""),
        "gap widened": lambda x: row(x, "benchy-on").__setitem__("poll_max_gap_ms", "300.0"),
        "t164 wall moved": lambda x: x[1]["cells"]["benchy-arachne-on"]["orca"].__setitem__("wall_median_s", 2.3),
        "findings edited": lambda x: x.__setitem__(4, x[4].replace("0.898x", "0.798x")) if isinstance(x, list) else None,
    }
    rejected = 0
    for name, fn in controls.items():
        x = list(copy.deepcopy(base))
        try:
            fn(x)
            errs = checks(*x)
        except Exception as e:  # a crash on a mutated input is a rejection
            errs = [repr(e)]
        ok = bool(errs)
        rejected += ok
        print(f"{'rejected' if ok else 'ACCEPTED'}: {name}")
    print(f"{rejected}/{len(controls)} negative controls rejected")
    return 0 if rejected == len(controls) else 1


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()
    errs = checks(*load())
    for e in errs:
        print("FAIL:", e)
    print("PASS" if not errs else f"FAIL ({len(errs)})")
    return 1 if errs else 0


if __name__ == "__main__":
    raise SystemExit(main())
