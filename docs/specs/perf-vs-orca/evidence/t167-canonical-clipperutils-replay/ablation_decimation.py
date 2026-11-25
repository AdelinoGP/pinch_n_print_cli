"""Decompose orca_v2_5_0's offset speedup into (1) Rust 1.1 -> C++ 1.5.2,
(2) C++ 1.5.2 -> Clipper2 2.0.1 + ClipperUtils without decimation, and
(3) append_offset_path edge decimation (0.005 * |delta|).

Runs on corpus/base-prepass-t167.corpus (118 cases) and writes checkout-resident
results/ablation_decimation.json.
"""

import collections
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t167-canonical-clipperutils-replay"
CORPUS = HERE / "corpus/base-prepass-t167.corpus"
OUT_JSON = HERE / "results/ablation_decimation.json"


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def rel_area_delta(a: float, ref: float) -> float:
    denom = max(abs(a), abs(ref), 1.0)
    return abs(a - ref) / denom


def build_nodecim_binary() -> Path:
    src = (HERE / "driver_orca_v250.cpp").read_text(encoding="utf-8")
    target = "static constexpr double CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR = 0.005;"
    repl = "static constexpr double CLIPPER_OFFSET_SHORTEST_EDGE_FACTOR = 0.0;"
    assert target in src
    tmp_src = SCRATCH / "driver_orca_v250_nodecim.cpp"
    tmp_src.write_text(src.replace(target, repl, 1), encoding="utf-8")

    out_bin = SCRATCH / "driver-orca-v250-nodecim.exe"
    orca_c2 = Path(r"D:\slicerProject\Orca(pnp_gui)\deps_src\clipper2\Clipper2Lib")
    cmd = [
        "g++", "-O3", "-std=c++17", "-static",
        f"-I{orca_c2 / 'include'}",
        f"-I{HERE / 'shims'}",
        str(tmp_src),
        str(orca_c2 / "src/clipper.engine.cpp"),
        str(orca_c2 / "src/clipper.offset.cpp"),
        str(orca_c2 / "src/clipper.rectclip.cpp"),
        "-o", str(out_bin),
    ]
    subprocess.run(cmd, check=True)
    return out_bin


def run_batch(binary: str, corpus: Path):
    res = subprocess.run(
        [binary, str(corpus)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=300,
        check=True,
    )
    rows = [json.loads(line) for line in res.stdout.decode("utf-8").splitlines() if line.strip()]
    return rows


def main():
    prov = json.loads((SCRATCH / "provenance.json").read_text(encoding="utf-8"))
    nodecim_bin = build_nodecim_binary()
    bins = {
        "rust_pnp": prov["binaries"]["rust_pnp"]["path"],
        "cpp_c2_1_5_2": prov["binaries"]["cpp_c2_1_5_2"]["path"],
        "orca_v2_5_0_nodecim": str(nodecim_bin),
        "orca_v2_5_0": prov["binaries"]["orca_v2_5_0"]["path"],
    }

    # Forward + reverse batch average for each engine to cancel thermal drift
    runs = {}
    for order, seq in (
        ("forward", ("rust_pnp", "cpp_c2_1_5_2", "orca_v2_5_0_nodecim", "orca_v2_5_0")),
        ("reverse", ("orca_v2_5_0", "orca_v2_5_0_nodecim", "cpp_c2_1_5_2", "rust_pnp")),
    ):
        runs[order] = {}
        for eng in seq:
            runs[order][eng] = run_batch(bins[eng], CORPUS)
            print(f"[ablation/{order}] {eng}: {len(runs[order][eng])} cases", flush=True)

    n = len(runs["forward"]["rust_pnp"])
    cases = []
    by_site = collections.defaultdict(list)
    for i in range(n):
        ref = runs["forward"]["rust_pnp"][i]
        rec = {
            "case_index": i,
            "site": ref["site"],
            "layer": ref["layer"],
            "opcode": ref["opcode"],
            "in_verts": ref["in_verts"],
        }
        for eng in bins:
            f_row = runs["forward"][eng][i]
            r_row = runs["reverse"][eng][i]
            assert f_row["out_hash"] == r_row["out_hash"]
            # Combine all 10 timed repeats (5 forward + 5 reverse) via median
            all_10 = f_row["repeat_times_ns"] + r_row["repeat_times_ns"]
            rec[eng] = {
                "median_ns": int(statistics.median(all_10)),
                "fwd_median_ns": int(statistics.median(f_row["repeat_times_ns"])),
                "rev_median_ns": int(statistics.median(r_row["repeat_times_ns"])),
                "out_paths": f_row["out_paths"],
                "out_verts": f_row["out_verts"],
                "out_area_u2": f_row["out_area_u2"],
                "out_hash": f_row["out_hash"],
            }
        cases.append(rec)
        by_site[rec["site"]].append(rec)

    site_summary = {}
    for site, items in sorted(by_site.items()):
        sums = {
            eng: round(sum(x[eng]["median_ns"] for x in items) / 1e6, 4)
            for eng in bins
        }
        meds = {
            eng: round(statistics.median(x[eng]["median_ns"] for x in items) / 1e6, 5)
            for eng in bins
        }
        def ratio(a, b):
            return round(sums[a] / sums[b], 4) if sums[b] > 0 else None

        area_nodecim = [rel_area_delta(x["orca_v2_5_0_nodecim"]["out_area_u2"], x["rust_pnp"]["out_area_u2"]) for x in items]
        area_decim = [rel_area_delta(x["orca_v2_5_0"]["out_area_u2"], x["rust_pnp"]["out_area_u2"]) for x in items]
        site_summary[site] = {
            "n": len(items),
            "sum_ms": sums,
            "median_ms": meds,
            "factors": {
                "total_rust_over_orca_v250": ratio("rust_pnp", "orca_v2_5_0"),
                "f1_rust_over_cpp152": ratio("rust_pnp", "cpp_c2_1_5_2"),
                "f2_cpp152_over_c201_nodecim": ratio("cpp_c2_1_5_2", "orca_v2_5_0_nodecim"),
                "f3_c201_nodecim_over_c201_decim": ratio("orca_v2_5_0_nodecim", "orca_v2_5_0"),
            },
            "median_rel_area_vs_rust": {
                "orca_v2_5_0_nodecim": round(statistics.median(area_nodecim), 6),
                "orca_v2_5_0": round(statistics.median(area_decim), 6),
            },
            "max_rel_area_vs_rust": {
                "orca_v2_5_0_nodecim": round(max(area_nodecim), 6),
                "orca_v2_5_0": round(max(area_decim), 6),
            },
        }

    payload = {
        "corpus": "t167/base-prepass-t167.corpus",
        "corpus_sha256": sha256_file(CORPUS),
        "case_count": n,
        "method": "batch mode, forward+reverse (10 repeats median per case), single-thread",
        "by_site": site_summary,
        "cases": cases,
    }
    OUT_JSON.parent.mkdir(parents=True, exist_ok=True)
    OUT_JSON.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print("\n=== 3-Way Factor Decomposition (sum_ms ratios) ===")
    for site, s in site_summary.items():
        f = s["factors"]
        print(
            f"{site:20s} total={f['total_rust_over_orca_v250']:6.2f}x = "
            f"rust/c152({f['f1_rust_over_cpp152']:4.2f}x) * "
            f"c152/c201_nodec({f['f2_cpp152_over_c201_nodecim']:4.2f}x) * "
            f"decim({f['f3_c201_nodecim_over_c201_decim']:4.2f}x) | "
            f"med_area nodec={s['median_rel_area_vs_rust']['orca_v2_5_0_nodecim']:.6f} "
            f"dec={s['median_rel_area_vs_rust']['orca_v2_5_0']:.6f}"
        )


if __name__ == "__main__":
    main()
