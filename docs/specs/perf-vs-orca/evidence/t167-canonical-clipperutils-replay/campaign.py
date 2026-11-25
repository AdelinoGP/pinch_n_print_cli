"""t167 four-engine replay campaign: single-thread (forward + reverse) and 12-way loaded."""

import concurrent.futures
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import sys
import time

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
T69 = HERE.parent / "t69-clipper-engine"
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t167-canonical-clipperutils-replay"
RAW_SINGLE = SCRATCH / "raw/single"
RAW_LOADED = SCRATCH / "raw/loaded12"

ENGINES_FWD = ("rust_pnp", "cpp_c2_1_5_2", "orca_v2_5_0", "orca_clipper1")
ENGINES_REV = tuple(reversed(ENGINES_FWD))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sample_cpu_load() -> float:
    try:
        out = subprocess.check_output(
            [
                "pwsh", "-NoProfile", "-Command",
                "(Get-CimInstance Win32_Processor | Measure-Object -Property LoadPercentage -Average).Average",
            ],
            text=True,
            timeout=15,
        ).strip()
        return float(out)
    except Exception:
        return -1.0


def load_corpus_files():
    files = []
    # 1) t69 existing corpus (8 files, 295 cases)
    for p in sorted((T69 / "corpus/t68sites").glob("*.corpus")):
        files.append(("t68sites/" + p.name, p))
    for p in sorted((T69 / "corpus/t55").glob("*.cases")):
        files.append(("t55/" + p.name, p))
    # 2) t167 new prepass corpus (apply_opening + gate_internal_bridge_sites on base classic-off)
    prepass = HERE / "corpus/base-prepass-t167.corpus"
    assert prepass.is_file(), f"Missing prepass corpus: {prepass}"
    files.append(("t167/" + prepass.name, prepass))
    return files


def parse_blocks(corpus_files):
    population = []
    for label, path in corpus_files:
        text = path.read_text()
        blocks = re.findall(r"^case\s.*?^end\s*$", text, re.M | re.S)
        assert blocks, f"No cases in {path}"
        for idx, block in enumerate(blocks):
            population.append((label, path, idx, block + "\n"))
    return population


def run_single(prov, population):
    RAW_SINGLE.mkdir(parents=True, exist_ok=True)
    binaries = {k: v["path"] for k, v in prov["binaries"].items()}
    total = len(population)
    for order, engines in (("forward", ENGINES_FWD), ("reverse", ENGINES_REV)):
        seq = population if order == "forward" else list(reversed(population))
        t0 = time.monotonic()
        for step, (label, path, idx, block) in enumerate(seq, 1):
            stem = label.replace("/", "__").replace(".", "_")
            d = RAW_SINGLE / order / stem / str(idx)
            d.mkdir(parents=True, exist_ok=True)
            inp = d / "input.case"
            if not inp.exists():
                inp.write_text(block)
            for eng in engines:
                out = d / f"{eng}.jsonl"
                if out.exists():
                    row = json.loads(out.read_text())
                    assert len(row["repeat_times_ns"]) == 5
                    continue
                res = subprocess.run(
                    [binaries[eng], str(inp)],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=180,
                    check=True,
                )
                lines = res.stdout.decode().splitlines()
                assert len(lines) == 1, (eng, label, idx, len(lines))
                row = json.loads(lines[0])
                assert len(row["repeat_times_ns"]) == 5
                out.write_bytes(res.stdout)
            if step % 50 == 0 or step == total:
                dt = time.monotonic() - t0
                print(f"[single/{order}] {step}/{total} ({dt:.1f}s)", flush=True)


def run_loaded12(prov, population, workers=12):
    """Run all base-model cases under 12-way concurrent process load.

    Each of the 12 simultaneous worker processes executes the exact same set of cases
    in a cyclically staggered order, so all 12 cores and memory bandwidth stay saturated
    throughout the run without running identical inputs in lockstep.
    """
    RAW_LOADED.mkdir(parents=True, exist_ok=True)
    binaries = {k: v["path"] for k, v in prov["binaries"].items()}

    # Select all base-model cases (excluding accelerated-base duplicate of ordinary-base)
    # so the 12-way loaded corpus covers all hot prepass, classic, region-prep, and union sites.
    selected = [
        (label, idx, block)
        for (label, _path, idx, block) in population
        if label in (
            "t167/base-prepass-t167.corpus",
            "t68sites/base-t68sites.corpus",
            "t68sites/base-region-prep.corpus",
            "t55/ordinary-base.cases",
        )
    ]
    n = len(selected)
    assert n >= workers, f"Too few cases for {workers} workers: {n}"

    # Write 12 cyclically staggered batch files (identical multiset of cases per worker)
    stride = max(1, n // workers)
    worker_files = []
    worker_maps = []
    batch_dir = RAW_LOADED / "batches"
    batch_dir.mkdir(parents=True, exist_ok=True)
    for w in range(workers):
        offset = (w * stride) % n
        ordered = selected[offset:] + selected[:offset]
        wf = batch_dir / f"worker_{w:02d}.cases"
        wf.write_text("\n".join(b for (_, _, b) in ordered))
        worker_files.append(wf)
        worker_maps.append([(lbl, idx) for (lbl, idx, _) in ordered])

    for order, engines in (("forward", ENGINES_FWD), ("reverse", ENGINES_REV)):
        for eng in engines:
            out_path = RAW_LOADED / f"{order}_{eng}.json"
            if out_path.exists():
                print(f"[loaded12/{order}] {eng} already exists, skipping", flush=True)
                continue
            t_start = time.monotonic()
            load_before = sample_cpu_load()

            def launch(w_idx):
                t0 = time.monotonic()
                res = subprocess.run(
                    [binaries[eng], str(worker_files[w_idx])],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    timeout=600,
                    check=True,
                )
                dt = time.monotonic() - t0
                rows = [json.loads(line) for line in res.stdout.decode().splitlines() if line.strip()]
                assert len(rows) == n, (eng, w_idx, len(rows), n)
                return w_idx, dt, rows

            with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as ex:
                futs = [ex.submit(launch, w) for w in range(workers)]
                worker_results = [f.result() for f in futs]

            wall_s = time.monotonic() - t_start
            load_after = sample_cpu_load()
            worker_walls = [dt for (_, dt, _) in worker_results]

            # Aggregate per original case across the 12 concurrent workers
            by_case = {}
            for w_idx, _dt, rows in worker_results:
                wmap = worker_maps[w_idx]
                for pos, row in enumerate(rows):
                    lbl, orig_idx = wmap[pos]
                    key = f"{lbl}::{orig_idx}"
                    rec = by_case.setdefault(
                        key,
                        {
                            "corpus": lbl,
                            "orig_index": orig_idx,
                            "site": row["site"],
                            "layer": row["layer"],
                            "opcode": row["opcode"],
                            "in_verts": row["in_verts"],
                            "out_paths": row["out_paths"],
                            "out_rings": row["out_rings"],
                            "out_verts": row["out_verts"],
                            "out_area_u2": row["out_area_u2"],
                            "out_hash": row["out_hash"],
                            "worker_medians_ns": [],
                        },
                    )
                    assert rec["out_hash"] == row["out_hash"], (eng, key)
                    rec["worker_medians_ns"].append(statistics.median(row["repeat_times_ns"]))

            payload = {
                "order": order,
                "engine": eng,
                "workers": workers,
                "case_count": n,
                "batch_wall_s": wall_s,
                "worker_wall_min_s": min(worker_walls),
                "worker_wall_median_s": statistics.median(worker_walls),
                "worker_wall_max_s": max(worker_walls),
                "cpu_load_before": load_before,
                "cpu_load_after": load_after,
                "cases": list(by_case.values()),
            }
            out_path.write_text(json.dumps(payload, indent=2) + "\n")
            print(
                f"[loaded12/{order}] {eng:14s} wall={wall_s:.2f}s "
                f"(workers {min(worker_walls):.2f}..{max(worker_walls):.2f}s)",
                flush=True,
            )


def main():
    prov = json.loads((SCRATCH / "provenance.json").read_text())
    # Verify binary hashes before running
    for eng, info in prov["binaries"].items():
        assert sha256(Path(info["path"])) == info["sha256"], f"Binary hash mismatch: {eng}"

    corpus_files = load_corpus_files()
    population = parse_blocks(corpus_files)
    manifest = {
        "provenance_sha256": sha256(SCRATCH / "provenance.json"),
        "corpus_files": {lbl: sha256(p) for lbl, p in corpus_files},
        "total_cases": len(population),
        "cpu_load_start": sample_cpu_load(),
    }
    (SCRATCH / "campaign_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Loaded {len(population)} cases across {len(corpus_files)} files (CPU load {manifest['cpu_load_start']}%)", flush=True)

    mode = sys.argv[1] if len(sys.argv) > 1 else "all"
    if mode in ("all", "single"):
        run_single(prov, population)
    if mode in ("all", "loaded12"):
        run_loaded12(prov, population, workers=12)

    # Verify binary hashes after running
    for eng, info in prov["binaries"].items():
        assert sha256(Path(info["path"])) == info["sha256"], f"Binary hash changed: {eng}"
    print("CAMPAIGN PASS", flush=True)


if __name__ == "__main__":
    main()
