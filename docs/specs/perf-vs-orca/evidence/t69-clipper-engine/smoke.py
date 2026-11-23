"""Run both engines and reduce actual JSONL; raw outputs remain gitignored."""

import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t69-clipper-engine"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--corpus", type=Path, default=HERE / "corpus/t55")
    parser.add_argument("--name", default="t55-smoke")
    parser.add_argument(
        "--reduce-only",
        action="store_true",
        help="reduce existing JSONL without rerunning engines",
    )
    parser.add_argument("--report", type=Path, default=HERE / "SMOKE.md")
    options = parser.parse_args()
    assert Path(options.name).name == options.name
    files = sorted(options.corpus.resolve().glob("*.cases"))
    assert files, "required corpus is absent"
    seal = json.loads((SCRATCH / "seal.json").read_text())
    assert seal["lock_seal_passed"] and seal["pristine_copy_byte_identical"]
    for relative, expected in seal["binaries"].items():
        assert sha(SCRATCH / relative) == expected, "binary changed since setup"
    for name, expected in seal["driver_sources"].items():
        assert sha(HERE / name) == expected, "driver changed since setup; rebuild"
    raw = SCRATCH / "raw" / options.name
    raw.mkdir(parents=True, exist_ok=True)
    results = {}
    commands = {}
    for engine, binary in (
        ("rust", SCRATCH / "rust-harness/target/release/t69-dual-engine.exe"),
        ("cpp", SCRATCH / "cpp-driver.exe"),
    ):
        command = [str(binary), *map(str, files)]
        commands[engine] = command
        if not options.reduce_only:
            print("RUN " + json.dumps(command), flush=True)
            with (
                (raw / f"{engine}.jsonl").open("w") as out,
                (raw / f"{engine}.stderr.log").open("w") as err,
            ):
                subprocess.run(
                    command,
                    cwd=SCRATCH,
                    stdout=out,
                    stderr=err,
                    check=True,
                    timeout=1800,
                )
        rows = [
            json.loads(line)
            for line in (raw / f"{engine}.jsonl").read_text().splitlines()
        ]
        assert rows and all(
            len(r["repeat_times_ns"]) == 5 and all(t > 0 for t in r["repeat_times_ns"])
            for r in rows
        )
        results[engine] = {(r["file"], r["case_index"]): r for r in rows}
        assert len(results[engine]) == len(rows), "duplicate case identity"
    assert results["rust"].keys() == results["cpp"].keys(), (
        "unmatched engine populations"
    )
    summary = []
    for opcode in sorted({r["opcode"] for r in results["rust"].values()}):
        keys = [k for k, r in results["rust"].items() if r["opcode"] == opcode]
        for key in keys:
            a, b = results["rust"][key], results["cpp"][key]
            assert all(a[field] == b[field] for field in ("site", "layer", "opcode"))
        medians = {
            engine: statistics.median(
                statistics.median(results[engine][k]["repeat_times_ns"]) for k in keys
            )
            for engine in results
        }
        agreements = sum(
            results["rust"][k]["out_hash"] == results["cpp"][k]["out_hash"]
            for k in keys
        )
        shape_matches = sum(
            all(
                results["rust"][k][field] == results["cpp"][k][field]
                for field in ("out_paths", "out_rings", "out_verts")
            )
            for k in keys
        )
        summary.append(
            {
                "opcode": opcode,
                "cases": len(keys),
                "median_of_case_medians_ns": medians,
                "rust_over_cpp": medians["rust"] / medians["cpp"],
                "ordered_hash_agreements": agreements,
                "ordered_hash_divergences": len(keys) - agreements,
                "shape_count_agreements": shape_matches,
            }
        )
    report = {
        "commands": commands,
        "summary": summary,
        "corpus": {str(f): sha(f) for f in files},
        "raw_results": {
            engine: {
                "path": str(raw / f"{engine}.jsonl"),
                "sha256": sha(raw / f"{engine}.jsonl"),
            }
            for engine in results
        },
        "seal_sha256": sha(SCRATCH / "seal.json"),
    }
    (raw / "summary.json").write_text(json.dumps(report, indent=2) + "\n")
    lines = [
        "# Dual-engine smoke measurement",
        "",
        "Measured actual results; nanoseconds. One untimed warm-up, then five repeats per case.",
        "Each engine entry is median(across cases, median(five repeats)); ratio is Rust / C++.",
        "",
        "| Opcode | Cases | Rust median ns | C++ median ns | Rust / C++ | Ordered hash match | Shape-count match |",
        "|---|---:|---:|---:|---:|---:|---:|",
    ]
    for r in summary:
        m = r["median_of_case_medians_ns"]
        lines.append(
            f"| {r['opcode']} | {r['cases']} | {m['rust']:,.1f} | {m['cpp']:,.1f} | {r['rust_over_cpp']:.4f} | {r['ordered_hash_agreements']}/{r['cases']} | {r['shape_count_agreements']}/{r['cases']} |"
        )
    for r in summary:
        lines += [
            "",
            f"Opcode {r['opcode']} ordered-hash divergences: {r['ordered_hash_divergences']} of {r['cases']} cases.",
        ]
    lines += [
        "",
        "## Method and limitations",
        "",
        "The t55 corpus includes all ordinary/accelerated base/benchy captures; mode-identical duplicates are deliberately retained, not independent workloads.",
        "Only union is covered by t55; no offset, difference, intersection or xor headroom is inferred from this run.",
        "Timing includes engine creation, AddSubject/AddClip or AddPaths, Execute into PolyTree, and engine destruction. It excludes parsing, input flattening, result hashing, tree traversal and output tree destruction.",
        "Fresh engines are constructed for every repeat. Repeat hashes/counts are checked against the warm-up result.",
        "Output rings are raw engine results grouped as outer + direct holes; nested islands precede enclosing outers. No coordinate, ring-start, orientation or component-order normalization is performed.",
        "Ordered coordinate hash disagreement is not a geometric equivalence verdict. Rust 1.1.0 vs canonical C++ 1.5.2 can differ (ticket 31 clean_collinear delta); divergences are disclosed, not investigated here.",
        "FNV-1a hashes use x then y as little-endian two's-complement u64 bytes for every vertex, no framing bytes. Shape counts are compared separately; collisions remain possible.",
        "These are microbenchmarks under different compilers, runtime allocators, and engine revisions. They size possible headroom, not like-for-like optimization speedups or slicer wall-time savings. Sequential execution is not a paired process-level benchmark and has no CPU isolation.",
        "",
        "## Toolchain and seal",
        "",
        "```text",
        seal["rustc"].strip(),
        seal["g++"].strip(),
        "```",
        "",
        "C++ uses g++ -O3 -std=c++17 and static runtime linking; Rust uses cargo build --release --locked --offline (default release optimization).",
        f"Crate package SHA-256: `{seal['crate_package_checksum']}`.",
        "Registry files match the lock-checksummed cached .crate archive; scratch copy is byte-identical. Own Cargo.lock resolves exactly clipper2-rust 1.1.0, with no registry source. Seal passed before and after build; root lock and canonical sources unchanged.",
        "",
        "## Raw results (gitignored)",
        "",
    ]
    for engine in results:
        lines.append(
            f"* {engine}: `{(raw / f'{engine}.jsonl').relative_to(REPO).as_posix()}` (SHA-256 `{sha(raw / f'{engine}.jsonl')}`)."
        )
    lines += [
        f"* Reduction: `{(raw / 'summary.json').relative_to(REPO).as_posix()}`.",
        f"* Build provenance and exact compiler argv/cwd: `{(SCRATCH / 'seal.json').relative_to(REPO).as_posix()}`.",
        "",
        "Reproduce with the commands in HARNESS.md. No production tests or end-to-end parity qualification were run.",
    ]
    controls = SCRATCH / "raw/protocol-controls/verification.json"
    if controls.exists():
        outcomes = json.loads(controls.read_text())
        lines += ["", "## Independent protocol validation", ""]
        for engine, result in outcomes.items():
            lines.append(
                f"* {engine}: {result['rectangle_cases_passed']} known-coordinate rectangle/empty-output controls passed; unknown argument rejected: {result['unknown_arg_rejected']}."
            )
        lines += [
            f"* Saved result: `{controls.relative_to(REPO).as_posix()}`. These controls cover all opcodes, negative coordinates, named site tokens, optional call index and the FNV byte definition; they are not performance evidence."
        ]
    options.report.write_text("\n".join(lines) + "\n")
    print(json.dumps(summary, indent=2), flush=True)


if __name__ == "__main__":
    main()
