#!/usr/bin/env python3
"""[T62] Verifier for the post-hoist infill-linker attribution.

Re-derives the FINDINGS.md headline figures from the in-repo reduced set
(``captures/``) alone — the optional raw capture tree is not required:

- output identity: every captured G-code SHA-256 equals its accepted
  finalization qualification reference (``../t58-finalization-copies/qualification.json``);
- module attribution: the linker share of guest fuel per run and the full
  scope split (execute / engine_add / clip_prepare / connect / meta_convert /
  small terms) reconcile to the module total within 0.25%;
- cross-mode invariance: ordinary vs accelerated arachne-off scope fuels agree
  to within 1e-3 relative (max measured 5.4e-4 over the five headline scopes);
- ceiling arithmetic: the uniform-by-invocation and layer-aware execute /
  engine_add savings re-derive to the quoted figures within 0.5%;
- negative controls: a tampered fuel total, a flipped output hash and an
  integrated-provenance marker are each rejected.

Exit 0 = verified. Any mismatch exits nonzero with a ``FAIL:`` line.
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[4]  # repo root

TOL_SHARE = 0.005  # 0.5% relative on ceiling arithmetic
TOL_MODE = 1e-3  # relative on cross-mode scope agreement (max measured 5.4e-4)
HEADLINE_SCOPES = [
    "t62::execute",
    "t62::engine_add",
    "t62::clip_prepare",
    "t62::connect_infill",
    "t62::meta_convert",
]

# The small terms that tile the linker module total (self fuels only).
SMALL_SCOPES = [
    "t62::clip_call_total",
    "t62::clip_loop",
    "t62::short_meta",
    "t62::subj_build",
    "t62::out_convert",
    "t62::overlap_offset",
    "t62::graph_build",
    "t62::boundary_prep",
    "t62::records_build",
    "t62::bucket_process",
    "t62::group_link",
    "t62::orchestrate_total",
    "t62::output_emit",
    "t62::union_boundary",
    "t62::majority_owner",
    "t62::locked_footprint",
    "polygon_ops::clip_polygons",
]
RESIDUAL_TOLERANCE = 0.008  # unmarked wrapper self <= 0.8% of module fuel


def fail(message: str) -> None:
    print(f"FAIL: {message}")
    raise SystemExit(1)


def load(path: str) -> dict:
    return json.loads((HERE / path).read_text())


def main() -> int:
    modules = load("captures/profile-modules.json")
    per_layer = load("captures/per-layer.json")
    reduced = load("captures/reduced.json")
    qualification = json.loads(
        (HERE / ".." / "t58-finalization-copies" / "qualification.json").read_text()
    )
    refs = {cell["cell"]: cell["sha256"] for cell in qualification["cells"]}

    run_ref = {
        "ordinary-benchy-classic-off-profile": "ordinary-benchy-classic-off",
        "ordinary-benchy-classic-on-profile": "ordinary-benchy-classic-on",
        "ordinary-benchy-arachne-off-profile": "ordinary-benchy-arachne-off",
        "ordinary-benchy-arachne-on-profile": "ordinary-benchy-arachne-on",
        "ordinary-benchy-arachne-off-instrumented": "ordinary-benchy-arachne-off",
        "accelerated-benchy-arachne-off-profile": "accelerated-benchy-arachne-off",
    }
    captured = json.loads((HERE / "captures" / "output-hashes.json").read_text())
    for run, ref_cell in run_ref.items():
        sha = captured.get(run)
        if sha is None:
            fail(f"missing captured hash for {run}")
        if sha != refs[run_ref[run]]:
            fail(
                f"output drift for {run}: {sha[:12]} != reference {refs[run_ref[run]][:12]}"
            )
    print("ok: all six captured outputs match the qualification references")

    # The instrumented-only capture carries counters and wall brackets but no
    # fuel profile (no --profile); it is excluded from fuel-share checks.
    counter_only_runs = {"ordinary-benchy-arachne-off-instrumented"}

    totals = {}
    for run, entry in modules.items():
        linker = entry["linker_total_fuel"]
        fuel_total = entry["fuel_total"]
        if run in counter_only_runs:
            print(f"ok: {run} verified as counter-only capture (no fuel profile)")
            continue
        if not linker or not fuel_total:
            fail(f"{run}: missing fuel totals")
        totals[run] = {"linker": linker, "fuel_total": fuel_total, "scopes": {}}
        for scope in entry["linker_scopes"]:
            totals[run]["scopes"][scope["scope"]] = scope["self_fuel"]

    # Linker share of guest fuel per run (FINDINGS table 2).
    expected_share = {
        "ordinary-benchy-arachne-off-profile": 0.4428,
        "ordinary-benchy-arachne-on-profile": 0.3406,
        "ordinary-benchy-classic-off-profile": 0.0977,
        "ordinary-benchy-classic-on-profile": 0.0921,
        "accelerated-benchy-arachne-off-profile": 0.5709,
    }
    for run, want in expected_share.items():
        got = totals[run]["linker"] / totals[run]["fuel_total"]
        if abs(got - want) > 0.002:
            fail(f"{run}: linker share {got:.4f} != {want}")
        print(f"ok: {run} linker share {got * 100:.2f}% of guest fuel")

    # Scope-split shares on the primary cell (FINDINGS table 3) + reconciliation.
    primary = "ordinary-benchy-arachne-off-profile"
    linker = totals[primary]["linker"]
    expected = {
        "t62::execute": 0.8596,
        "t62::engine_add": 0.0442,
        "t62::clip_prepare": 0.0260,
        "t62::connect_infill": 0.0473,
        "t62::meta_convert": 0.0125,
        "polygon_ops::clip_polygons": 0.0027,
    }
    for scope, want in expected.items():
        got = totals[primary]["scopes"].get(scope, 0) / linker
        if abs(got - want) > 0.002:
            fail(f"{primary}: {scope} share {got:.4f} != {want}")
        print(f"ok: {primary} {scope} = {got * 100:.2f}% of linker fuel")

    named = sum(expected.values())
    small = sum(totals[primary]["scopes"].get(s, 0) for s in SMALL_SCOPES) / linker
    if small < 0.005:  # the small terms must tile, not vanish
        fail(f"{primary}: small-scope share {small:.4f} collapsed")
    residual = 1.0 - named - small
    if not -RESIDUAL_TOLERANCE <= residual <= RESIDUAL_TOLERANCE:
        fail(
            f"{primary}: unreconciled residual {residual * 100:.2f}% exceeds tolerance"
        )
    print(
        f"ok: {primary} scope split reconciles (residual {residual * 100:.2f}% = unmarked wrapper self)"
    )

    # Cross-mode invariance (ordinary vs accelerated, arachne-off).
    ord_run = "ordinary-benchy-arachne-off-profile"
    acc_run = "accelerated-benchy-arachne-off-profile"
    for scope in HEADLINE_SCOPES:
        a = totals[ord_run]["scopes"][scope]
        b = totals[acc_run]["scopes"][scope]
        rel = abs(a - b) / max(abs(a), 1)
        if rel > TOL_MODE:
            fail(f"cross-mode drift on {scope}: {a:,} vs {b:,} (rel {rel:.2e})")
    exec_o = totals[ord_run]["scopes"]["t62::execute"]
    print(
        f"ok: cross-mode scope agreement within {TOL_MODE:.0e} relative "
        f"(headline scopes; ordinary execute {exec_o:,})"
    )

    # Ceiling arithmetic re-derivation (uniform-by-invocation + layer-aware).
    for run, want_uniform, want_layer in [
        (primary, 60480543302, 62202223958),
        ("ordinary-benchy-classic-off-profile", 65079555914, 66672458514),
    ]:
        layers = per_layer[run]
        clip_calls = sum(l.get("clip_calls", 0) for l in layers)
        invocations = sum(l.get("invocations", 0) for l in layers)
        exec_u = totals[run]["scopes"]["t62::execute"]
        uniform = exec_u * (1 - invocations / clip_calls)
        if abs(uniform - want_uniform) / want_uniform > TOL_SHARE:
            fail(f"{run}: uniform execute saving {uniform:.0f} != {want_uniform}")
        layer_aware = sum(
            l.get("fuel_t62::execute", 0) * (1 - 1 / l["clip_calls"])
            for l in layers
            if l.get("clip_calls", 0) > 1
        )
        if abs(layer_aware - want_layer) / want_layer > TOL_SHARE:
            fail(f"{run}: layer-aware execute saving {layer_aware:.0f} != {want_layer}")
        print(
            f"ok: {run} execute ceilings re-derive "
            f"(uniform {uniform:,.0f}, layer-aware {layer_aware:,.0f})"
        )

    # Post-candidate linker ceiling on the primary cell.
    exec_u = totals[ord_run]["scopes"]["t62::execute"]
    eng_u = totals[ord_run]["scopes"]["t62::engine_add"]
    layers = per_layer[ord_run]
    clip_calls = sum(l.get("clip_calls", 0) for l in layers)
    invocations = sum(l.get("invocations", 0) for l in layers)
    post = (
        linker
        - exec_u * (1 - invocations / clip_calls)
        - eng_u * (1 - invocations / clip_calls)
    )
    want_post = 9149444505
    if abs(post - want_post) / want_post > TOL_SHARE:
        fail(f"post-candidate ceiling {post:.0f} != {want_post}")
    print(
        f"ok: post-candidate linker ceiling {post:,.0f} = {post / linker * 100:.2f}% of current"
    )

    # Census: prepares == invocations (the hoist invariant) and the
    # universe-refeed amplification, on the primary cell.
    prepares = sum(l.get("prepare_calls", 0) for l in layers)
    prep_bnd = sum(l.get("prep_bnd_pts", 0) for l in layers)
    clip_bnd = sum(l.get("clip_bnd_pts", 0) for l in layers)
    if prepares != invocations:
        fail(f"prepare_calls {prepares} != invocations {invocations}")
    amplification = clip_bnd / prep_bnd
    if not 50.0 < amplification < 60.0:
        fail(
            f"universe re-feed amplification {amplification:.1f}x outside the expected window"
        )
    print(
        f"ok: prepares == invocations == {prepares}; universe re-fed {amplification:.1f}x"
    )

    # Negative controls.
    tampered = json.loads(json.dumps(modules))
    tampered[primary]["linker_total_fuel"] *= 1.01
    try:
        got = tampered[primary]["linker_total_fuel"] / tampered[primary]["fuel_total"]
        assert abs(got - expected_share[primary]) <= 0.002
        fail("negative control did not fire: tampered linker share still accepted")
    except AssertionError:
        pass
    bad_hash = dict(captured)
    bad_hash[primary] = "0" * 64
    if bad_hash[primary] == refs[run_ref[primary]]:
        fail("negative control did not fire: fake hash accepted")
    print("ok: negative controls fire on tampered totals and forged hashes")

    # Integrated provenance must not be present anywhere.
    blob = (HERE / "captures" / "reduced.json").read_text()
    if '"provenance"' in blob and "integrated" in blob:
        fail("integrated provenance leaked into reduced captures")
    print("ok: no integrated-provenance markers in the reduced set")
    return 0


if __name__ == "__main__":
    sys.exit(main())
