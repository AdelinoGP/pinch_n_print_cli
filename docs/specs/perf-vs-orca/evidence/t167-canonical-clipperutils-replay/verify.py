"""Checkout-only verification for Ticket 167 evidence.

Never reads .local-artifacts/ and never executes engine binaries.
Run normally or with --self-test to verify the reduced results, seals,
determinism, summary re-derivation, and negative-control rejection.
"""

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import subprocess
import sys

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from analyze import (  # noqa: E402
    DRIVER_FILES,
    ENGINES,
    RESULTS,
    T69,
    compute_summary,
    load_json,
    sha256_file,
    sha256_json,
)

CONTROLS = (
    "changed-timing",
    "deleted-case",
    "shifted-site",
    "output-drift",
    "changed-decision",
    "tampered-seal",
)


def load_checkout_corpus_population():
    files = []
    for p in sorted((T69 / "corpus/t68sites").glob("*.corpus")):
        files.append(("t68sites/" + p.name, p))
    for p in sorted((T69 / "corpus/t55").glob("*.cases")):
        files.append(("t55/" + p.name, p))
    prepass = HERE / "corpus/base-prepass-t167.corpus"
    assert prepass.is_file(), f"Missing prepass corpus: {prepass}"
    files.append(("t167/" + prepass.name, prepass))

    expected = {}
    file_hashes = {}
    for label, path in files:
        file_hashes[label] = sha256_file(path)
        text = path.read_text(encoding="utf-8")
        blocks = re.findall(r"^case\s.*?^end\s*$", text, re.M | re.S)
        assert blocks, f"Empty corpus: {label}"
        for idx, block in enumerate(blocks):
            hdr = block.splitlines()[0].split()
            expected[(label, idx)] = (hdr[1], int(hdr[2]), int(hdr[3]))
    return file_hashes, expected


def validate(s_fwd, s_rev, l_fwd, l_rev, summary, provenance, census, ablation=None):
    # 1. Verify driver source seals
    for rel, expected_sha in provenance["driver_sources_sha256"].items():
        p = HERE / rel
        assert p.is_file(), f"Missing sealed driver file: {rel}"
        assert sha256_file(p) == expected_sha, f"Driver seal mismatch: {rel}"
    for f in DRIVER_FILES:
        assert f in provenance["driver_sources_sha256"], f"Unsealed driver file: {f}"

    # 2. Verify corpus seals and prepass census
    file_hashes, expected_cases = load_checkout_corpus_population()
    assert provenance["corpus_files_sha256"] == file_hashes, "Corpus SHA-256 mismatch"
    assert provenance["campaign_case_count"] == len(expected_cases), "Campaign case count mismatch"
    assert census["corpus_sha256"] == file_hashes["t167/base-prepass-t167.corpus"], "Prepass census hash mismatch"
    assert census["matches_t166_reference"] is True
    assert census["capture_gcode_sha256"].startswith("0a1ee3df67e5e971")

    # 3. Verify reduced result integrity seals
    seals = provenance["reduced_results_sha256"]
    assert sha256_json(s_fwd) == seals["single_forward"], "single_forward integrity mismatch"
    assert sha256_json(s_rev) == seals["single_reverse"], "single_reverse integrity mismatch"
    assert sha256_json(l_fwd) == seals["loaded12_forward"], "loaded12_forward integrity mismatch"
    assert sha256_json(l_rev) == seals["loaded12_reverse"], "loaded12_reverse integrity mismatch"
    assert sha256_json(summary) == seals["summary"], "summary integrity mismatch"

    # 4. Verify single-thread case population, schema, and forward/reverse determinism
    indexed_single = {}
    for order, rows in (("forward", s_fwd), ("reverse", s_rev)):
        assert len(rows) == len(expected_cases), f"Row count mismatch in single_{order}"
        keys = [(r["corpus"], r["case_index"]) for r in rows]
        assert len(keys) == len(set(keys)) and set(keys) == set(expected_cases), f"Case set mismatch in single_{order}"
        indexed_single[order] = dict(zip(keys, rows))
        for key, r in zip(keys, rows):
            exp_site, exp_layer, exp_op = expected_cases[key]
            assert (r["site"], r["layer"], r["opcode"]) == (exp_site, exp_layer, exp_op), (
                f"Case identity mismatch: {order} {key}"
            )
            assert r["in_verts"] >= 0
            for eng in ENGINES:
                e = r[eng]
                assert math.isfinite(e["median_ns"]) and e["median_ns"] >= 0
                assert e["min_ns"] <= e["median_ns"] <= e["max_ns"]
                assert 0 <= e["zero_repeats"] <= 5
                assert len(e["raw_sha256"]) == 64
                assert len(e["out_hash"]) == 16 and all(c in "0123456789abcdef" for c in e["out_hash"])
                assert 0 <= e["out_paths"] <= e["out_rings"] <= e["out_verts"]
                assert math.isfinite(e["out_area_u2"]) and e["out_area_u2"] >= 0.0

    for key in expected_cases:
        for eng in ENGINES:
            a = indexed_single["forward"][key][eng]
            b = indexed_single["reverse"][key][eng]
            for fld in ("out_hash", "out_paths", "out_rings", "out_verts", "out_area_u2"):
                assert a[fld] == b[fld], f"Forward/reverse output drift: {key} {eng} {fld}"

    # 5. Verify 12-way loaded population, determinism, and identity with single-thread outputs
    indexed_loaded = {}
    for order, payload in (("forward", l_fwd), ("reverse", l_rev)):
        assert payload["order"] == order
        rows = payload["cases"]
        assert len(rows) == provenance["loaded12_case_count"]
        for eng in ENGINES:
            m = payload["engines"][eng]
            assert m["workers"] == 12
            assert m["case_count"] == len(rows)
            assert m["batch_wall_s"] > 0 and m["worker_wall_min_s"] <= m["worker_wall_median_s"] <= m["worker_wall_max_s"]
        keys = [(r["corpus"], r["case_index"]) for r in rows]
        assert len(keys) == len(set(keys))
        indexed_loaded[order] = dict(zip(keys, rows))
        for key, r in zip(keys, rows):
            assert key in indexed_single["forward"], f"Loaded case not in single population: {key}"
            for eng in ENGINES:
                e = r[eng]
                s = indexed_single["forward"][key][eng]
                assert e["min_ns"] <= e["median_ns"] <= e["max_ns"]
                for fld in ("out_hash", "out_paths", "out_rings", "out_verts", "out_area_u2"):
                    assert e[fld] == s[fld], f"Loaded/single output drift: {order} {key} {eng} {fld}"
    assert set(indexed_loaded["forward"].keys()) == set(indexed_loaded["reverse"].keys())

    # 6. Re-derive summary.json verbatim from reduced cases
    rederived = compute_summary(s_fwd, s_rev, l_fwd, l_rev)
    assert rederived == summary, "Re-derived summary.json mismatch"

    # 7. Verify ablation results (required and sealed)
    assert ablation is not None, "Missing ablation_decimation.json"
    assert sha256_json(ablation) == seals["ablation_decimation"], "ablation_decimation integrity mismatch"
    assert ablation["case_count"] == census["total_selected_cases"]
    assert ablation["corpus_sha256"] == file_hashes["t167/base-prepass-t167.corpus"]
    assert len(ablation["cases"]) == ablation["case_count"]
    by_site = {}
    for c in ablation["cases"]:
        by_site.setdefault(c["site"], []).append(c)
    for site, items in by_site.items():
        rec = ablation["by_site"][site]
        assert rec["n"] == len(items)
        for eng in ("rust_pnp", "cpp_c2_1_5_2", "orca_v2_5_0_nodecim", "orca_v2_5_0"):
            got_ms = round(sum(x[eng]["median_ns"] for x in items) / 1e6, 4)
            assert got_ms == rec["sum_ms"][eng], f"Ablation sum mismatch: {site} {eng}"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--self-test", action="store_true", help="Run all negative controls")
    parser.add_argument("--negative-control", choices=CONTROLS)
    args = parser.parse_args()

    s_fwd = load_json(RESULTS / "single_forward.json")
    s_rev = load_json(RESULTS / "single_reverse.json")
    l_fwd = load_json(RESULTS / "loaded12_forward.json")
    l_rev = load_json(RESULTS / "loaded12_reverse.json")
    summary = load_json(RESULTS / "summary.json")
    provenance = load_json(RESULTS / "provenance.json")
    census = load_json(HERE / "corpus/prepass_census.json")
    abl_path = RESULTS / "ablation_decimation.json"
    ablation = load_json(abl_path) if abl_path.is_file() else None

    if args.negative_control == "changed-timing":
        s_fwd[0]["rust_pnp"]["median_ns"] *= 2
        provenance["reduced_results_sha256"]["single_forward"] = sha256_json(s_fwd)
    elif args.negative_control == "deleted-case":
        s_fwd.pop()
        provenance["reduced_results_sha256"]["single_forward"] = sha256_json(s_fwd)
    elif args.negative_control == "shifted-site":
        s_fwd[0]["site"] = "BOGUS_SITE"
        provenance["reduced_results_sha256"]["single_forward"] = sha256_json(s_fwd)
    elif args.negative_control == "output-drift":
        s_rev[0]["orca_v2_5_0"]["out_hash"] = "0000000000000000"
        provenance["reduced_results_sha256"]["single_reverse"] = sha256_json(s_rev)
    elif args.negative_control == "changed-decision":
        summary["decision_branch"] = "parity_within_1_5x"
        provenance["reduced_results_sha256"]["summary"] = sha256_json(summary)
    elif args.negative_control == "tampered-seal":
        s_fwd[0]["orca_v2_5_0"]["median_ns"] += 1

    try:
        validate(s_fwd, s_rev, l_fwd, l_rev, summary, provenance, census, ablation)
    except (AssertionError, KeyError, ValueError) as exc:
        print("FAIL:", str(exc))
        return 1

    if args.negative_control:
        print("ERROR: negative control survived:", args.negative_control)
        return 0

    if args.self_test:
        rejected = []
        for ctrl in CONTROLS:
            res = subprocess.run(
                [sys.executable, str(HERE / "verify.py"), "--negative-control", ctrl],
                capture_output=True,
                text=True,
                timeout=30,
            )
            assert res.returncode == 1 and "FAIL:" in res.stdout, (ctrl, res.returncode, res.stdout, res.stderr)
            rejected.append(ctrl)
        print("Negative controls rejected (6/6):", ", ".join(rejected))

    print("Decision branch:", summary["decision_branch"])
    print("Cases verified:", provenance["campaign_case_count"], "single,", provenance["loaded12_case_count"], "loaded12")
    print("t167 checkout-only verify: PASS (exit 0)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
