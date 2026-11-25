"""Reduce raw t167 campaign logs into checkout-resident results/ and re-derivable summary.json."""

import collections
import hashlib
import json
import math
from pathlib import Path
import statistics

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T69 = HERE.parent / "t69-clipper-engine"
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t167-canonical-clipperutils-replay"
RAW_SINGLE = SCRATCH / "raw/single"
RAW_LOADED = SCRATCH / "raw/loaded12"
RESULTS = HERE / "results"

ENGINES = ("rust_pnp", "cpp_c2_1_5_2", "orca_v2_5_0", "orca_clipper1")
OPCODE_NAMES = {
    0: "offset",
    1: "difference",
    2: "union",
    3: "intersection",
    4: "xor",
    5: "opening_ex",
}
DRIVER_FILES = (
    "driver_rust.rs",
    "driver_cpp_c2.cpp",
    "driver_orca_v250.cpp",
    "driver_orca_c1.cpp",
    "shims/oneapi/tbb/scalable_allocator.h",
    "shims/Eigen/Geometry",
    "setup_engines.py",
    "verify_drivers.py",
    "campaign.py",
    "reduce_prepass.py",
    "ablation_decimation.py",
)

# Hot prepass sites on base classic-off directly tested by Ticket 167's decision rule
HOT_PREPASS_SITES = (
    "SH_OPEN_TOP_CANON",
    "SH_OPEN_BOT_CANON",
    "SH_OPEN_TOP_LEGACY",
    "SH_OPEN_BOT_LEGACY",
    "IB_DEEP_POS",
    "IB_DEEP_NEG",
)


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sha256_json(obj) -> str:
    return hashlib.sha256(json.dumps(obj, sort_keys=True).encode("utf-8")).hexdigest()


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def rel_area_delta(a: float, ref: float) -> float:
    denom = max(abs(a), abs(ref), 1.0)
    return abs(a - ref) / denom


def safe_ratio(num: float, den: float):
    if den <= 0.0:
        return None
    return round(num / den, 4)


def percentile(vals, p: float) -> float:
    s = sorted(vals)
    if not s:
        return 0.0
    idx = min(len(s) - 1, max(0, int(round(p * (len(s) - 1)))))
    return s[idx]


def summarize_group(rows, mode="single"):
    """Compute per-engine timing sums, medians, ratios, and geometry deltas for a group of cases."""
    n = len(rows)
    assert n > 0
    per_eng = {}
    for eng in ENGINES:
        meds = [r[eng]["median_ns"] for r in rows]
        per_eng[eng] = {
            "sum_ms": round(sum(meds) / 1e6, 4),
            "median_ms": round(statistics.median(meds) / 1e6, 5),
            "p90_ms": round(percentile(meds, 0.9) / 1e6, 5),
            "max_ms": round(max(meds) / 1e6, 5),
        }

    # Ratios of summed medians (workload-weighted) AND median of per-case ratios
    ratios = {}
    pairs = (
        ("rust_vs_orca_v250", "rust_pnp", "orca_v2_5_0"),
        ("rust_vs_cpp_152", "rust_pnp", "cpp_c2_1_5_2"),
        ("rust_vs_orca_c1", "rust_pnp", "orca_clipper1"),
        ("cpp_152_vs_orca_v250", "cpp_c2_1_5_2", "orca_v2_5_0"),
        ("orca_v250_vs_orca_c1", "orca_v2_5_0", "orca_clipper1"),
    )
    for label, a_eng, b_eng in pairs:
        sum_a = sum(r[a_eng]["median_ns"] for r in rows)
        sum_b = sum(r[b_eng]["median_ns"] for r in rows)
        case_ratios = [
            r[a_eng]["median_ns"] / r[b_eng]["median_ns"]
            for r in rows
            if r[b_eng]["median_ns"] > 0
        ]
        ratios[label] = {
            "sum_ratio": safe_ratio(sum_a, sum_b),
            "median_case_ratio": round(statistics.median(case_ratios), 4) if case_ratios else None,
        }

    # Geometry agreement vs rust_pnp
    geom_vs_rust = {}
    for eng in ("cpp_c2_1_5_2", "orca_v2_5_0", "orca_clipper1"):
        rel_areas = [
            rel_area_delta(r[eng]["out_area_u2"], r["rust_pnp"]["out_area_u2"])
            for r in rows
        ]
        vert_ratios = [
            r[eng]["out_verts"] / r["rust_pnp"]["out_verts"]
            for r in rows
            if r["rust_pnp"]["out_verts"] > 0
        ]
        exact_zero_both = sum(
            1 for r in rows
            if r[eng]["out_paths"] == 0 and r["rust_pnp"]["out_paths"] == 0
        )
        nonempty_mismatch = sum(
            1 for r in rows
            if (r[eng]["out_paths"] == 0) != (r["rust_pnp"]["out_paths"] == 0)
        )
        geom_vs_rust[eng] = {
            "median_rel_area_delta": round(statistics.median(rel_areas), 6),
            "p90_rel_area_delta": round(percentile(rel_areas, 0.9), 6),
            "max_rel_area_delta": round(max(rel_areas), 6),
            "median_verts_ratio": round(statistics.median(vert_ratios), 4) if vert_ratios else None,
            "both_empty": exact_zero_both,
            "empty_mismatch": nonempty_mismatch,
        }

    return {
        "n": n,
        "in_verts_median": int(statistics.median(r["in_verts"] for r in rows)),
        "engines": per_eng,
        "ratios": ratios,
        "geom_vs_rust": geom_vs_rust,
    }


def summarize_pass(rows):
    """Group rows by overall, corpus, opcode, and site (deduplicating t55 accelerated mirror for overall/opcode)."""
    groups = {}
    # Deduplicated unique cases (exclude accelerated-*.cases mirrors of ordinary-*.cases from aggregate opcodes)
    unique_rows = [r for r in rows if not r["corpus"].startswith("t55/accelerated-")]
    groups["all_unique"] = summarize_group(unique_rows)
    groups["all_raw"] = summarize_group(rows)

    by_corpus = collections.defaultdict(list)
    by_op = collections.defaultdict(list)
    by_site = collections.defaultdict(list)
    by_op_prepass = collections.defaultdict(list)

    for r in rows:
        by_corpus[r["corpus"]].append(r)
    for r in unique_rows:
        by_op[OPCODE_NAMES[r["opcode"]]].append(r)
        by_site[r["site"]].append(r)
        if r["corpus"] == "t167/base-prepass-t167.corpus":
            by_op_prepass[OPCODE_NAMES[r["opcode"]]].append(r)

    for k, v in sorted(by_corpus.items()):
        groups[f"corpus/{k}"] = summarize_group(v)
    for k, v in sorted(by_op.items()):
        groups[f"opcode/{k}"] = summarize_group(v)
    for k, v in sorted(by_op_prepass.items()):
        groups[f"prepass_opcode/{k}"] = summarize_group(v)
    for k, v in sorted(by_site.items()):
        groups[f"site/{k}"] = summarize_group(v)
    return groups


def compute_summary(single_fwd, single_rev, loaded_fwd, loaded_rev):
    """Pure re-derivable summary from the four reduced checkout JSONs."""
    s_fwd = summarize_pass(single_fwd)
    s_rev = summarize_pass(single_rev)
    l_fwd = summarize_pass(loaded_fwd["cases"])
    l_rev = summarize_pass(loaded_rev["cases"])

    # Forward/reverse spreads on single-thread rust_vs_orca_v250 and rust_vs_orca_c1 sum_ratio
    single_spread = {}
    for k in s_fwd:
        single_spread[k] = {
            pair: round(
                abs(
                    (s_fwd[k]["ratios"][pair]["sum_ratio"] or 0.0)
                    - (s_rev[k]["ratios"][pair]["sum_ratio"] or 0.0)
                ),
                4,
            )
            for pair in ("rust_vs_orca_v250", "rust_vs_cpp_152", "rust_vs_orca_c1")
        }

    # Loaded/single scaling per engine on the shared base-model subset
    loaded_keys = {(c["corpus"], c["case_index"]) for c in loaded_fwd["cases"]}
    single_base_fwd = [r for r in single_fwd if (r["corpus"], r["case_index"]) in loaded_keys]
    single_base_rev = [r for r in single_rev if (r["corpus"], r["case_index"]) in loaded_keys]
    sb_fwd = summarize_pass(single_base_fwd)
    sb_rev = summarize_pass(single_base_rev)

    scaling = {}
    for order, l_grp, sb_grp in (("forward", l_fwd, sb_fwd), ("reverse", l_rev, sb_rev)):
        sc_ord = {}
        for k in l_grp:
            if k not in sb_grp:
                continue
            sc_ord[k] = {
                eng: safe_ratio(
                    l_grp[k]["engines"][eng]["sum_ms"],
                    sb_grp[k]["engines"][eng]["sum_ms"],
                )
                for eng in ENGINES
            }
        scaling[order] = sc_ord

    # Loaded batch-level wall summary
    batch_wall = {}
    for order, payload in (("forward", loaded_fwd), ("reverse", loaded_rev)):
        bw = {}
        for eng in ENGINES:
            b = payload["engines"][eng]
            bw[eng] = {
                "batch_wall_s": round(b["batch_wall_s"], 4),
                "worker_wall_median_s": round(b["worker_wall_median_s"], 4),
                "worker_wall_min_s": round(b["worker_wall_min_s"], 4),
                "worker_wall_max_s": round(b["worker_wall_max_s"], 4),
                "cpu_load_before": b["cpu_load_before"],
                "cpu_load_after": b["cpu_load_after"],
            }
        bw["ratios_vs_orca_v250"] = {
            eng: safe_ratio(bw[eng]["batch_wall_s"], bw["orca_v2_5_0"]["batch_wall_s"])
            for eng in ENGINES
        }
        batch_wall[order] = bw

    # Hot prepass site decision evaluation
    hot_table = {}
    for site in HOT_PREPASS_SITES:
        k = f"site/{site}"
        if k in s_fwd:
            hot_table[site] = {
                "single_fwd_rust_vs_v250": s_fwd[k]["ratios"]["rust_vs_orca_v250"]["sum_ratio"],
                "single_rev_rust_vs_v250": s_rev[k]["ratios"]["rust_vs_orca_v250"]["sum_ratio"],
                "loaded12_fwd_rust_vs_v250": l_fwd[k]["ratios"]["rust_vs_orca_v250"]["sum_ratio"],
                "loaded12_rev_rust_vs_v250": l_rev[k]["ratios"]["rust_vs_orca_v250"]["sum_ratio"],
                "single_fwd_rust_vs_c1": s_fwd[k]["ratios"]["rust_vs_orca_c1"]["sum_ratio"],
                "single_rev_rust_vs_c1": s_rev[k]["ratios"]["rust_vs_orca_c1"]["sum_ratio"],
                "loaded12_fwd_rust_vs_c1": l_fwd[k]["ratios"]["rust_vs_orca_c1"]["sum_ratio"],
                "loaded12_rev_rust_vs_c1": l_rev[k]["ratios"]["rust_vs_orca_c1"]["sum_ratio"],
            }

    hot_v250_vals = [
        min(v["single_fwd_rust_vs_v250"], v["single_rev_rust_vs_v250"])
        for v in hot_table.values()
        if v["single_fwd_rust_vs_v250"] is not None
    ]
    all_ge_3x = all(v >= 3.0 for v in hot_v250_vals) if hot_v250_vals else False
    any_ge_3x = any(v >= 3.0 for v in hot_v250_vals) if hot_v250_vals else False
    all_le_1_5x = all(v <= 1.5 for v in hot_v250_vals) if hot_v250_vals else False

    if all_ge_3x:
        branch = "canonical_ge_3x_faster_all_hot_prepass_sites"
    elif any_ge_3x:
        branch = "canonical_ge_3x_faster_on_hot_offset_sites"
    elif all_le_1_5x:
        branch = "parity_within_1_5x"
    else:
        branch = "intermediate_1_5x_to_3x"

    return {
        "single": {"forward": s_fwd, "reverse": s_rev, "spread_sum_ratio": single_spread},
        "single_base_subset": {"forward": sb_fwd, "reverse": sb_rev},
        "loaded12": {"forward": l_fwd, "reverse": l_rev, "batch_wall": batch_wall},
        "loaded12_over_single_scaling": scaling,
        "hot_prepass_sites": hot_table,
        "decision_branch": branch,
    }


def build_reduced_from_raw():
    """Read scratch raw outputs and emit checkout-resident results/*.json."""
    RESULTS.mkdir(parents=True, exist_ok=True)
    prov_raw = load_json(SCRATCH / "provenance.json")
    manifest = load_json(SCRATCH / "campaign_manifest.json")

    from campaign import load_corpus_files, parse_blocks

    corpus_files = load_corpus_files()
    population = parse_blocks(corpus_files)

    single_out = {}
    for order in ("forward", "reverse"):
        rows = []
        for label, _path, idx, _block in population:
            stem = label.replace("/", "__").replace(".", "_")
            d = RAW_SINGLE / order / stem / str(idx)
            rec = None
            for eng in ENGINES:
                p = d / f"{eng}.jsonl"
                raw_bytes = p.read_bytes()
                r = json.loads(raw_bytes.decode("utf-8"))
                times = r["repeat_times_ns"]
                assert len(times) == 5
                if rec is None:
                    rec = {
                        "corpus": label,
                        "case_index": idx,
                        "site": r["site"],
                        "layer": r["layer"],
                        "opcode": r["opcode"],
                        "in_verts": r["in_verts"],
                    }
                else:
                    assert rec["site"] == r["site"]
                    assert rec["layer"] == r["layer"]
                    assert rec["opcode"] == r["opcode"]
                    assert rec["in_verts"] == r["in_verts"]
                eng_rec = {
                    "median_ns": int(statistics.median(times)),
                    "min_ns": int(min(times)),
                    "max_ns": int(max(times)),
                    "zero_repeats": sum(1 for t in times if t == 0),
                    "out_paths": r["out_paths"],
                    "out_rings": r["out_rings"],
                    "out_verts": r["out_verts"],
                    "out_area_u2": r["out_area_u2"],
                    "out_hash": r["out_hash"],
                    "raw_sha256": hashlib.sha256(raw_bytes).hexdigest(),
                }
                if "stray_holes" in r:
                    eng_rec["stray_holes"] = r["stray_holes"]
                rec[eng] = eng_rec
            rows.append(rec)
        single_out[order] = rows
        (RESULTS / f"single_{order}.json").write_text(json.dumps(rows, indent=2) + "\n")

    loaded_out = {}
    for order in ("forward", "reverse"):
        eng_meta = {}
        by_key = {}
        for eng in ENGINES:
            p = RAW_LOADED / f"{order}_{eng}.json"
            raw_bytes = p.read_bytes()
            d = json.loads(raw_bytes.decode("utf-8"))
            eng_meta[eng] = {
                "workers": d["workers"],
                "case_count": d["case_count"],
                "batch_wall_s": d["batch_wall_s"],
                "worker_wall_min_s": d["worker_wall_min_s"],
                "worker_wall_median_s": d["worker_wall_median_s"],
                "worker_wall_max_s": d["worker_wall_max_s"],
                "cpu_load_before": d["cpu_load_before"],
                "cpu_load_after": d["cpu_load_after"],
                "raw_sha256": hashlib.sha256(raw_bytes).hexdigest(),
            }
            for c in d["cases"]:
                key = (c["corpus"], c["orig_index"])
                rec = by_key.setdefault(
                    key,
                    {
                        "corpus": c["corpus"],
                        "case_index": c["orig_index"],
                        "site": c["site"],
                        "layer": c["layer"],
                        "opcode": c["opcode"],
                        "in_verts": c["in_verts"],
                    },
                )
                wms = c["worker_medians_ns"]
                rec[eng] = {
                    "median_ns": int(statistics.median(wms)),
                    "min_ns": int(min(wms)),
                    "max_ns": int(max(wms)),
                    "out_paths": c["out_paths"],
                    "out_rings": c["out_rings"],
                    "out_verts": c["out_verts"],
                    "out_area_u2": c["out_area_u2"],
                    "out_hash": c["out_hash"],
                }
        cases_sorted = [by_key[k] for k in sorted(by_key.keys())]
        payload = {"order": order, "engines": eng_meta, "cases": cases_sorted}
        loaded_out[order] = payload
        (RESULTS / f"loaded12_{order}.json").write_text(json.dumps(payload, indent=2) + "\n")

    summary = compute_summary(
        single_out["forward"], single_out["reverse"], loaded_out["forward"], loaded_out["reverse"]
    )
    (RESULTS / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")

    abl_path = RESULTS / "ablation_decimation.json"
    reduced_seals = {
        "single_forward": sha256_json(single_out["forward"]),
        "single_reverse": sha256_json(single_out["reverse"]),
        "loaded12_forward": sha256_json(loaded_out["forward"]),
        "loaded12_reverse": sha256_json(loaded_out["reverse"]),
        "summary": sha256_json(summary),
    }
    if abl_path.is_file():
        reduced_seals["ablation_decimation"] = sha256_json(load_json(abl_path))

    # Checkout-resident provenance (no absolute machine paths required for verification)
    prov_checkout = {
        "schema_version": 1,
        "campaign_case_count": len(population),
        "loaded12_case_count": len(loaded_out["forward"]["cases"]),
        "loaded12_workers": 12,
        "cpu_load_start_pct": manifest["cpu_load_start"],
        "sources": {
            "pnp_git_rev": prov_raw["pnp_git_rev"],
            "pnp_clipper2_rust": "vendor/clipper2-rust 1.1.0 (t70-optimized)",
            "orca_v250_git_rev": prov_raw["orca_v250_git_rev"],
            "orca_v250_clipper2_version": "2.0.1",
            "orca_v250_clipper2_migration_commit": "222c6a2df50e4dee7139ff7bf27b7fac56a36c5d",
            "orca_documented_git_rev": prov_raw["orca_documented_git_rev"],
            "orca_documented_clipper1_version": "6.2.6",
            "orca_documented_clipper2_version": "1.5.2",
        },
        "compilers": {
            "rustc": prov_raw["rustc"].splitlines()[0],
            "rustc_full": prov_raw["rustc"],
            "gxx": prov_raw["gxx"].splitlines()[0],
            "cxx_flags": prov_raw["cxx_flags"],
            "rust_profile": "release (cargo build --release --offline)",
        },
        "allocator_disclosure": {
            "rust_pnp": "mimalloc::MiMalloc (matching pnp_cli production global allocator)",
            "cpp_c2_1_5_2": "MinGW g++ default CRT malloc (no tbbmalloc)",
            "orca_v2_5_0": "MinGW g++ default CRT malloc (tbb::scalable_allocator shimmed to std::malloc/std::allocator)",
            "orca_clipper1": "MinGW g++ default CRT malloc (tbb::scalable_allocator shimmed to std::malloc/std::allocator)",
            "note": "Any allocator advantage favors rust_pnp (mimalloc) over the C++ shims; C++ speedups are conservative lower bounds vs Orca's native tbbmalloc build.",
        },
        "binaries_sha256": {k: v["sha256"] for k, v in prov_raw["binaries"].items()},
        "driver_sources_sha256": {f: sha256_file(HERE / f) for f in DRIVER_FILES},
        "corpus_files_sha256": manifest["corpus_files"],
        "reduced_results_sha256": reduced_seals,
    }
    (RESULTS / "provenance.json").write_text(json.dumps(prov_checkout, indent=2) + "\n")
    print("Reduced results written to", RESULTS)
    print("Decision branch:", summary["decision_branch"])
    print("Hot prepass sites (rust_pnp / orca_v2_5_0, single fwd/rev | loaded12 fwd/rev):")
    for s, info in summary["hot_prepass_sites"].items():
        print(
            f"  {s:20s}: single={info['single_fwd_rust_vs_v250']}/{info['single_rev_rust_vs_v250']}x "
            f"| loaded12={info['loaded12_fwd_rust_vs_v250']}/{info['loaded12_rev_rust_vs_v250']}x "
            f"| vs_c1={info['single_fwd_rust_vs_c1']}/{info['single_rev_rust_vs_c1']}x"
        )


if __name__ == "__main__":
    build_reduced_from_raw()
