#!/usr/bin/env python3
"""t171 verification: re-derive every quoted number from the committed set.

Checkout-only and stdlib-only: it reads the committed `reduced/` artifacts plus
the sibling tickets' committed artifacts, recomputes the whole arithmetic chain
with `arithmetic.py`, and compares the result leaf by leaf.

    python verify.py              # 0 = every claim re-derives
    python verify.py --self-test  # also proves each check rejects tampering

The self-test mutates one input per control (an instrumented share, a paired
saving, a `--profile` scope, the S5 census, the S6 family, a run hash, a
validation flag, a stage bracket) and asserts the corresponding check fails.
"""

from __future__ import annotations

import argparse
import copy
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
EVID = HERE.parent

sys.path.insert(0, str(HERE))
import arithmetic  # noqa: E402

CELLS = arithmetic.CELLS
TOL = 2e-3


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def base_inputs() -> dict:
    return {
        "instrumented": load(HERE / "reduced" / "instrumented.json"),
        "profile_clean": load(HERE / "reduced" / "profile-clean.json"),
        "t169_ab": load(
            EVID / "t169-offset-decimation-flat-opening" / "reduced" / "ab.json"
        ),
        "t169_profile": load(
            EVID / "t169-offset-decimation-flat-opening" / "reduced" / "profile.json"
        ),
        "t163": load(EVID / "t163-accelerated-board" / "reduced" / "summary.json"),
        "t131": load(EVID / "t131-per-op-decomposition" / "summary.json"),
        "t169_reference": load(
            EVID / "t169-offset-decimation-flat-opening" / "reference.json"
        ),
        "runs": load(HERE / "runs.json"),
    }


def recompute(ctx: dict) -> dict:
    split = arithmetic.split_section(ctx["instrumented"])
    route = arithmetic.route(ctx["t169_ab"], ctx["t163"])
    ratios = arithmetic.rescale(ctx["t169_profile"])
    return {
        "split": split,
        "route": route,
        "offset_ratios": ratios,
        "stage_budget": arithmetic.stage_budget(split, route),
        "s5": arithmetic.s5_reprice(split, route, ratios, ctx["t131"]),
        "s6": arithmetic.s6_reprice(route, ratios, ctx["t131"]),
        "profile_scopes": {
            cell: ctx["profile_clean"]["cells"][cell]["scopes"] for cell in CELLS
        },
    }


def flatten(value, prefix="") -> dict:
    if isinstance(value, dict):
        out = {}
        for k, v in value.items():
            out.update(flatten(v, f"{prefix}/{k}"))
        return out
    if isinstance(value, list):
        out = {}
        for i, v in enumerate(value):
            out.update(flatten(v, f"{prefix}/{i}"))
        return out
    return {prefix: value}


def mismatches(stored: dict, fresh: dict) -> list[str]:
    a = flatten(stored)
    b = flatten(fresh)
    bad = []
    for key in sorted(set(a) | set(b)):
        if key not in a or key not in b:
            bad.append(f"missing {key}")
            continue
        x, y = a[key], b[key]
        if isinstance(x, (int, float)) and isinstance(y, (int, float)):
            if abs(x - y) > TOL * max(1.0, abs(y)):
                bad.append(f"{key}: stored {x} != re-derived {y}")
        elif x != y:
            bad.append(f"{key}: stored {x!r} != re-derived {y!r}")
    return bad


def check_payload() -> list[str]:
    stored = load(HERE / "reduced" / "arithmetic.json")
    fresh = recompute(base_inputs())
    fresh = {k: v for k, v in fresh.items() if k in stored}
    bad = mismatches({k: stored[k] for k in fresh}, fresh)
    # The note is prose; only its presence is asserted.
    if not stored.get("note"):
        bad.append("arithmetic.json carries no note")
    return bad


def check_runs() -> list[str]:
    ctx = base_inputs()
    refs = ctx["t169_reference"]["active_set"]
    bad = []
    seen = set()
    for row in ctx["runs"]:
        key = (row["cell"], row["pass"])
        seen.add(row["cell"])
        if row["exit_code"] != 0:
            bad.append(f"{key}: exit {row['exit_code']}")
        want = refs.get(row["cell"])
        if want is None:
            bad.append(f"{key}: no reference hash for cell")
        elif row["gcode_sha256"] != want:
            bad.append(
                f"{key}: gcode {row['gcode_sha256'][:12]} != reference {want[:12]}"
            )
    if len(ctx["runs"]) != 24:
        bad.append(f"expected 24 instrumented runs, found {len(ctx['runs'])}")
    if seen != set(CELLS):
        bad.append(f"cells covered {sorted(seen)} != {sorted(CELLS)}")
    return bad


def check_instrumented() -> list[str]:
    ctx = base_inputs()
    refs = ctx["t169_reference"]["active_set"]
    bad = []
    for cell in CELLS:
        rec = ctx["instrumented"]["cells"][cell]
        if rec["repeat_count"] != 3:
            bad.append(f"{cell}: {rec['repeat_count']} passes, want 3")
        if rec["median"]["gcode_sha256"] != refs[cell]:
            bad.append(f"{cell}: median hash != t169 reference")
        for i, p in enumerate(rec["passes"], start=1):
            if p["degraded"] is not False:
                bad.append(f"{cell} pass{i}: degraded={p['degraded']}")
            if p["fatal_error_count"] or p["non_fatal_error_count"]:
                bad.append(
                    f"{cell} pass{i}: errors fatal={p['fatal_error_count']} "
                    f"non-fatal={p['non_fatal_error_count']}"
                )
            if p["gcode_sha256"] != refs[cell]:
                bad.append(f"{cell} pass{i}: hash != t169 reference")
            if p["layer_count"] <= 0:
                bad.append(f"{cell} pass{i}: layer_count={p['layer_count']}")
    return bad


def check_reconciliation() -> list[str]:
    """Phase shares must sum to <= 1 and the stage brackets must reconcile."""
    ctx = base_inputs()
    bad = []
    for cell in CELLS:
        p = ctx["instrumented"]["cells"][cell]["passes"][0]
        share_sum = sum(arithmetic_shares(p).values())
        if share_sum > 1.0 + 1e-6:
            bad.append(f"{cell}: phase shares sum to {share_sum:.6f}")
        stage_sum = sum(p["prepass_stages_ms"].values())
        prepass = p["phases_ms"]["prepass"]
        if prepass and abs(stage_sum - prepass) / prepass > 0.05:
            bad.append(
                f"{cell}: stage brackets {stage_sum} ms vs prepass {prepass} ms "
                f"({100 * (stage_sum - prepass) / prepass:+.1f}%)"
            )
    return bad


def arithmetic_shares(pass_row: dict) -> dict:
    total = pass_row["slice_complete_ms"]
    out = {k: v / total for k, v in pass_row["phases_ms"].items()}
    out["outside"] = pass_row["outside_phases_ms"] / total
    return out


CHECKS = {
    "payload_recomputes": check_payload,
    "runs_match_reference": check_runs,
    "instrumented_valid": check_instrumented,
    "brackets_reconcile": check_reconciliation,
}


def run_checks() -> int:
    failures = 0
    for name, fn in CHECKS.items():
        bad = fn()
        if bad:
            failures += 1
            print(f"FAIL {name}")
            for line in bad[:12]:
                print(f"  - {line}")
            if len(bad) > 12:
                print(f"  ... {len(bad) - 12} more")
        else:
            print(f"PASS {name}")
    return failures


def self_test() -> int:
    """Each control mutates one input and must be rejected."""
    controls = []

    def payload_control(mutate, label="payload_recomputes"):
        def run() -> bool:
            ctx = base_inputs()
            mutate(ctx)
            stored = load(HERE / "reduced" / "arithmetic.json")
            fresh = recompute(ctx)
            fresh = {k: v for k, v in fresh.items() if k in stored}
            return bool(mismatches({k: stored[k] for k in fresh}, fresh))

        return (label, run)

    controls.append(
        payload_control(
            label="payload: instrumented share",
            mutate=lambda c: c["instrumented"]["cells"]["base-classic-off"]["median"][
                "shares"
            ].__setitem__(
                "prepass",
                c["instrumented"]["cells"]["base-classic-off"]["median"]["shares"][
                    "prepass"
                ]
                + 0.05,
            )
        )
    )
    controls.append(
        payload_control(
            label="payload: t169 paired saving",
            mutate=lambda c: c["t169_ab"]["base-classic-off"]["wall"].__setitem__(
                "candidate_median_s",
                c["t169_ab"]["base-classic-off"]["wall"]["candidate_median_s"] * 1.1,
            )
        )
    )
    controls.append(
        payload_control(
            label="payload: t169 offset profile",
            mutate=lambda c: c["t169_profile"]["arms"]["candidate"]["base-classic-off"][
                "scopes"
            ]["wall"].__setitem__(
                "polygon_ops::offset",
                c["t169_profile"]["arms"]["candidate"]["base-classic-off"]["scopes"][
                    "wall"
                ]["polygon_ops::offset"]
                * 2,
            )
        )
    )
    controls.append(
        payload_control(
            label="payload: t131 S5 census",
            mutate=lambda c: c["t131"]["cells"]["base-classic-off@s5"]["s5"].__setitem__(
                "sum_whole_pos_ms",
                c["t131"]["cells"]["base-classic-off@s5"]["s5"]["sum_whole_pos_ms"] * 2,
            )
        )
    )

    def mutate_s6(c: dict) -> None:
        fam = next(
            k
            for k in c["t131"]["cells"]["base-classic-on"]["s6_families"]
            if k.startswith("0.350000") and "none" in k
        )
        c["t131"]["cells"]["base-classic-on"]["s6_families"][fam]["repeat_calls"] *= 2

    controls.append(payload_control(mutate_s6, label="payload: t131 S6 family"))

    def runs_control() -> bool:
        ctx = base_inputs()
        ctx["runs"][0]["gcode_sha256"] = "0" * 64
        saved_runs, saved_load = None, None
        # check_runs reads from disk; emulate by patching its loader result.
        original = base_inputs

        def patched() -> dict:
            out = original()
            out["runs"] = ctx["runs"]
            return out

        globals()["base_inputs"] = patched
        try:
            return bool(check_runs())
        finally:
            globals()["base_inputs"] = original
        return False

    controls.append(("runs_match_reference", runs_control))

    def validation_control() -> bool:
        original = base_inputs

        def patched() -> dict:
            out = original()
            out["instrumented"] = copy.deepcopy(out["instrumented"])
            out["instrumented"]["cells"]["benchy-classic-off"]["passes"][1][
                "degraded"
            ] = True
            return out

        globals()["base_inputs"] = patched
        try:
            return bool(check_instrumented())
        finally:
            globals()["base_inputs"] = original
        return False

    controls.append(("instrumented_valid", validation_control))

    def bracket_control() -> bool:
        original = base_inputs

        def patched() -> dict:
            out = original()
            out["instrumented"] = copy.deepcopy(out["instrumented"])
            out["instrumented"]["cells"]["base-classic-off"]["passes"][0][
                "prepass_stages_ms"
            ]["PrePass::ShellClassification"] *= 2
            return out

        globals()["base_inputs"] = patched
        try:
            return bool(check_reconciliation())
        finally:
            globals()["base_inputs"] = original
        return False

    controls.append(("brackets_reconcile", bracket_control))

    failures = 0
    for name, fn in controls:
        rejected = fn()
        print(f"{'PASS' if rejected else 'FAIL'} self-test rejects {name} tampering")
        if not rejected:
            failures += 1
    print(
        f"self-test: {len(controls) - failures}/{len(controls)} negative controls rejected"
    )
    return failures


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--self-test", action="store_true")
    a = ap.parse_args()
    failures = run_checks()
    if a.self_test:
        failures += self_test()
    if failures:
        print(f"FAIL ({failures} check(s) failed)")
        return 1
    print("PASS")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
