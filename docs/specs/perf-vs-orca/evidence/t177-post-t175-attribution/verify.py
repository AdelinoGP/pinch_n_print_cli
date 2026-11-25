#!/usr/bin/env python3
"""t177 verification: checkout-only, stdlib-only re-derivation of every claim.

Reads committed `reduced/` artifacts plus sibling tickets' committed artifacts,
recomputes `reduced/arithmetic.json` leaf-by-leaf via `arithmetic.py`, and
verifies run hashes, validation flags, code-1200 counts, profile reconciliation,
bracket reconciliation, and cross-consistency with `t175/reduced/route.json`.

Usage:
  python verify.py              # 0 = all checks pass
  python verify.py --self-test  # also proves each check rejects tampering
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
        "profile_detail": load(HERE / "reduced" / "profile-detail-clean.json"),
        "t174": load(EVID / "t174-post-t172-scoreboard" / "reduced" / "summary.json"),
        "t175_ab": load(EVID / "t175-tiled-collision-carve" / "reduced" / "ab.json"),
        "t175_accel": load(
            EVID / "t175-tiled-collision-carve" / "reduced" / "accel.json"
        ),
        "t175_route": load(
            EVID / "t175-tiled-collision-carve" / "reduced" / "route.json"
        ),
        "t175_reference": load(
            EVID / "t175-tiled-collision-carve" / "reference.json"
        ),
        "t171": load(EVID / "t171-post-t169-reprice" / "reduced" / "arithmetic.json"),
        "runs": load(HERE / "runs.json"),
    }


def recompute(ctx: dict) -> dict:
    return arithmetic.compute_all(
        ctx["instrumented"],
        ctx["profile_clean"],
        ctx["profile_detail"],
        ctx["t174"],
        ctx["t175_ab"],
        ctx["t175_accel"],
        ctx["t171"],
    )


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
        if isinstance(x, bool) or isinstance(y, bool):
            if x != y:
                bad.append(f"{key}: stored {x!r} != re-derived {y!r}")
        elif isinstance(x, (int, float)) and isinstance(y, (int, float)):
            if abs(x - y) > TOL * max(1.0, abs(y)):
                bad.append(f"{key}: stored {x} != re-derived {y}")
        elif x != y:
            bad.append(f"{key}: stored {x!r} != re-derived {y!r}")
    return bad


def check_payload(ctx: dict | None = None) -> list[str]:
    ctx = base_inputs() if ctx is None else ctx
    stored = load(HERE / "reduced" / "arithmetic.json")
    fresh = recompute(ctx)
    bad = mismatches(
        {k: stored[k] for k in fresh if k != "note" and k in stored},
        {k: fresh[k] for k in fresh if k != "note"},
    )
    if not stored.get("note"):
        bad.append("arithmetic.json carries no note")
    # Cross-check route ratios against t175/reduced/route.json.
    for cell in CELLS:
        r177 = fresh["route"][cell]
        r175 = ctx["t175_route"]["cells"][cell]
        if abs(r177["est_post_t175_ratio_ord"] - r175["est_post_t175_ratio_ordinary"]) > 5e-4:
            bad.append(
                f"{cell}: ord route ratio {r177['est_post_t175_ratio_ord']} != "
                f"t175 {r175['est_post_t175_ratio_ordinary']}"
            )
        if abs(r177["est_post_t175_ratio_acc"] - r175["est_post_t175_ratio_accelerated"]) > 5e-4:
            bad.append(
                f"{cell}: acc route ratio {r177['est_post_t175_ratio_acc']} != "
                f"t175 {r175['est_post_t175_ratio_accelerated']}"
            )
    return bad


def check_runs(ctx: dict | None = None) -> list[str]:
    ctx = base_inputs() if ctx is None else ctx
    refs = ctx["t175_reference"]["active_set"]
    ref_bytes = ctx["t175_reference"]["bytes"]
    doc = ctx["runs"]
    ids = doc.get("identities", {})
    bad = []
    if ids.get("ordinary_guests_check_exit") != 0:
        bad.append(f"ordinary_guests_check_exit={ids.get('ordinary_guests_check_exit')}")
    if not ids.get("exe_sha256") or len(ids["exe_sha256"]) != 64:
        bad.append("missing/invalid exe_sha256 in runs.json identities")
    mods = ids.get("modules", {})
    if mods.get("module_count") != 24 or len(mods.get("wasm_tree_sha256", "")) != 64:
        bad.append(f"unexpected module identity: count={mods.get('module_count')}")

    rows = doc.get("runs", [])
    seen: dict[str, set[int]] = {}
    for row in rows:
        cell = row["cell"]
        seen.setdefault(cell, set()).add(row["pass"])
        key = (cell, row["pass"])
        if row["exit_code"] != 0:
            bad.append(f"{key}: exit {row['exit_code']}")
        if row["gcode_sha256"] != refs.get(cell):
            bad.append(f"{key}: gcode sha mismatch")
        if row["gcode_bytes"] != ref_bytes.get(cell):
            bad.append(f"{key}: gcode bytes {row['gcode_bytes']} != {ref_bytes.get(cell)}")
        if row.get("degraded") is not False:
            bad.append(f"{key}: degraded={row.get('degraded')}")
        if row.get("fatal_error_count") != 0 or row.get("non_fatal_error_count") != 0:
            bad.append(f"{key}: fatal/non_fatal errors present")
        if row.get("code_1200_count") != 0:
            bad.append(f"{key}: code_1200_count={row.get('code_1200_count')}")
    if len(rows) != 24:
        bad.append(f"expected 24 instrumented runs, found {len(rows)}")
    if set(seen) != set(CELLS) or any(v != {1, 2, 3} for v in seen.values()):
        bad.append(f"incomplete cell/pass coverage: {seen}")
    return bad


def check_instrumented(ctx: dict | None = None) -> list[str]:
    ctx = base_inputs() if ctx is None else ctx
    refs = ctx["t175_reference"]["active_set"]
    ref_bytes = ctx["t175_reference"]["bytes"]
    bad = []
    for cell in CELLS:
        rec = ctx["instrumented"]["cells"][cell]
        if rec["repeat_count"] != 3:
            bad.append(f"{cell}: {rec['repeat_count']} passes, want 3")
        if rec["median"]["gcode_sha256"] != refs[cell]:
            bad.append(f"{cell}: median hash != t175 reference")
        if rec["median"]["gcode_bytes"] != ref_bytes[cell]:
            bad.append(f"{cell}: median bytes != t175 reference")
        for i, p in enumerate(rec["passes"], start=1):
            if p["degraded"] is not False:
                bad.append(f"{cell} pass{i}: degraded={p['degraded']}")
            if p["fatal_error_count"] or p["non_fatal_error_count"]:
                bad.append(
                    f"{cell} pass{i}: errors fatal={p['fatal_error_count']} "
                    f"non-fatal={p['non_fatal_error_count']}"
                )
            if p.get("code_1200_count") != 0:
                bad.append(f"{cell} pass{i}: code_1200_count={p.get('code_1200_count')}")
            if p["gcode_sha256"] != refs[cell] or p["gcode_bytes"] != ref_bytes[cell]:
                bad.append(f"{cell} pass{i}: hash/bytes != t175 reference")
            if p["layer_count"] <= 0:
                bad.append(f"{cell} pass{i}: layer_count={p['layer_count']}")
    return bad


def check_profile(ctx: dict | None = None) -> list[str]:
    ctx = base_inputs() if ctx is None else ctx
    refs = ctx["t175_reference"]["active_set"]
    ref_bytes = ctx["t175_reference"]["bytes"]
    clean = ctx["profile_clean"]["cells"]
    detail = ctx["profile_detail"]["cells"]
    bad = []
    for cell in CELLS:
        c = clean[cell]
        d = detail[cell]
        if c["exit_code"] != 0:
            bad.append(f"{cell}: profile exit={c['exit_code']}")
        if not c["reference_match"] or c["gcode_sha256"] != refs[cell]:
            bad.append(f"{cell}: profile gcode hash != t175 reference")
        if c["gcode_bytes"] != ref_bytes[cell]:
            bad.append(f"{cell}: profile gcode bytes != t175 reference")
        if c["code_1200_count"] != 0:
            bad.append(f"{cell}: profile code_1200_count={c['code_1200_count']}")
        # Reconcile summed scopes between profile-clean.json and profile-detail-clean.json
        for section, group_key in (("fuel", "modules"), ("wall", "builtins")):
            summed: dict[str, int] = {}
            for row in d[section][group_key].values():
                for sname, srow in row["scopes"].items():
                    summed[sname] = summed.get(sname, 0) + srow["amount"]
            if summed != c["scopes"][section]:
                bad.append(
                    f"{cell} {section}: detail scope sums != profile-clean scopes"
                )
        if not d["fuel"]["total"] or d["fuel"]["total"] <= 0:
            bad.append(f"{cell}: invalid fuel total {d['fuel']['total']}")
    return bad


def arithmetic_shares(pass_row: dict) -> dict:
    total = pass_row["slice_complete_ms"]
    out = {k: v / total for k, v in pass_row["phases_ms"].items()}
    out["outside"] = pass_row["outside_phases_ms"] / total
    return out


def check_reconciliation(ctx: dict | None = None) -> list[str]:
    """Phase shares must sum to 1 and stage brackets must reconcile on all 24 passes."""
    ctx = base_inputs() if ctx is None else ctx
    bad = []
    for cell in CELLS:
        for idx, p in enumerate(ctx["instrumented"]["cells"][cell]["passes"], start=1):
            share_sum = sum(arithmetic_shares(p).values())
            if abs(share_sum - 1.0) > 1e-6:
                bad.append(f"{cell} pass{idx}: phase shares sum to {share_sum:.6f}")
            pre_sum = sum(p["prepass_stages_ms"].values())
            pre_wall = p["phases_ms"]["prepass"]
            if pre_wall and abs(pre_sum - pre_wall) / pre_wall > 0.05:
                bad.append(
                    f"{cell} pass{idx}: prepass stages {pre_sum} ms vs phase {pre_wall} ms "
                    f"({100 * (pre_sum - pre_wall) / pre_wall:+.1f}%)"
                )
            post_sum = sum(p["postpass_stages_ms"].values())
            post_wall = p["phases_ms"]["postpass"]
            if post_wall and abs(post_sum - post_wall) / post_wall > 0.05:
                bad.append(
                    f"{cell} pass{idx}: postpass stages {post_sum} ms vs phase {post_wall} ms "
                    f"({100 * (post_sum - post_wall) / post_wall:+.1f}%)"
                )
    return bad


CHECKS = {
    "payload_recomputes": check_payload,
    "runs_match_reference": check_runs,
    "instrumented_valid": check_instrumented,
    "profile_valid": check_profile,
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
    """Each negative control mutates one input and must be rejected."""
    controls = []

    def mutate_check(label, check_fn, mutator):
        def run() -> bool:
            ctx = copy.deepcopy(base_inputs())
            mutator(ctx)
            return bool(check_fn(ctx))

        return (label, run)

    controls.append(
        mutate_check(
            "payload: instrumented share",
            check_payload,
            lambda c: c["instrumented"]["cells"]["benchy-classic-on"]["median"][
                "shares"
            ].__setitem__(
                "prepass",
                c["instrumented"]["cells"]["benchy-classic-on"]["median"]["shares"][
                    "prepass"
                ]
                + 0.05,
            ),
        )
    )
    controls.append(
        mutate_check(
            "payload: t174 quiet-board ratio",
            check_payload,
            lambda c: c["t174"]["cells"]["base-classic-off"].__setitem__(
                "ratio_ordinary_over_orca",
                c["t174"]["cells"]["base-classic-off"]["ratio_ordinary_over_orca"]
                + 0.05,
            ),
        )
    )
    controls.append(
        mutate_check(
            "payload: t175 paired saving",
            check_payload,
            lambda c: c["t175_ab"]["base-classic-on"]["wall"].__setitem__(
                "median_saving_pct",
                c["t175_ab"]["base-classic-on"]["wall"]["median_saving_pct"] + 2.0,
            ),
        )
    )
    controls.append(
        mutate_check(
            "payload: profile host scope",
            check_payload,
            lambda c: c["profile_detail"]["cells"]["base-classic-on"]["wall"][
                "builtins"
            ]["host:native"]["scopes"]["polygon_ops::clip_polygons"].__setitem__(
                "amount",
                c["profile_detail"]["cells"]["base-classic-on"]["wall"]["builtins"][
                    "host:native"
                ]["scopes"]["polygon_ops::clip_polygons"]["amount"]
                * 2,
            ),
        )
    )
    controls.append(
        mutate_check(
            "payload: profile guest fuel",
            check_payload,
            lambda c: c["profile_detail"]["cells"]["benchy-classic-on"]["fuel"][
                "modules"
            ]["com.core.tree-support"].__setitem__(
                "amount",
                c["profile_detail"]["cells"]["benchy-classic-on"]["fuel"]["modules"][
                    "com.core.tree-support"
                ]["amount"]
                + 10_000_000_000,
            ),
        )
    )
    controls.append(
        mutate_check(
            "runs: gcode hash mismatch",
            check_runs,
            lambda c: c["runs"]["runs"][0].__setitem__("gcode_sha256", "0" * 64),
        )
    )
    controls.append(
        mutate_check(
            "runs: code-1200 diagnostic",
            check_runs,
            lambda c: c["runs"]["runs"][3].__setitem__("code_1200_count", 1),
        )
    )
    controls.append(
        mutate_check(
            "instrumented: degraded flag",
            check_instrumented,
            lambda c: c["instrumented"]["cells"]["benchy-classic-off"]["passes"][1].__setitem__(
                "degraded", True
            ),
        )
    )
    controls.append(
        mutate_check(
            "profile: reference mismatch",
            check_profile,
            lambda c: c["profile_clean"]["cells"]["base-classic-on"].__setitem__(
                "gcode_sha256", "f" * 64
            ),
        )
    )
    controls.append(
        mutate_check(
            "profile: detail vs clean scope drift",
            check_profile,
            lambda c: c["profile_clean"]["cells"]["base-classic-off"]["scopes"][
                "wall"
            ].__setitem__("polygon_ops::offset", 1),
        )
    )
    controls.append(
        mutate_check(
            "brackets: prepass stage reconciliation",
            check_reconciliation,
            lambda c: c["instrumented"]["cells"]["base-classic-off"]["passes"][2][
                "prepass_stages_ms"
            ].__setitem__(
                "PrePass::ShellClassification",
                c["instrumented"]["cells"]["base-classic-off"]["passes"][2][
                    "prepass_stages_ms"
                ]["PrePass::ShellClassification"]
                * 2,
            ),
        )
    )
    controls.append(
        mutate_check(
            "brackets: phase sum reconciliation",
            check_reconciliation,
            lambda c: c["instrumented"]["cells"]["benchy-arachne-on"]["passes"][0][
                "phases_ms"
            ].__setitem__(
                "per_layer",
                c["instrumented"]["cells"]["benchy-arachne-on"]["passes"][0][
                    "phases_ms"
                ]["per_layer"]
                + 500,
            ),
        )
    )

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
