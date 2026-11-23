"""Sealed, resumable per-case campaign. Run from any directory; no builds."""

import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t69-clipper-engine"
RAW = SCRATCH / "raw/t69-campaign"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def seal_check():
    seal = json.loads((SCRATCH / "seal.json").read_text())
    assert seal["lock_seal_passed"] and seal["pristine_copy_byte_identical"]
    for rel, expected in seal["binaries"].items():
        assert sha(SCRATCH / rel) == expected, rel
    for rel, expected in seal["driver_sources"].items():
        assert sha(HERE / rel) == expected, rel
    for root, inventory in [
        (SCRATCH / "clipper2-rust-1.1.0", seal["crate_inventory"]),
        (REPO / "OrcaSlicerDocumented/deps_src/clipper2", seal["cpp_inventory"]),
    ]:
        actual = {
            p.relative_to(root).as_posix(): sha(p)
            for p in root.rglob("*")
            if p.is_file()
        }
        assert actual == inventory, str(root)
    lock = SCRATCH / "rust-harness/Cargo.lock"
    assert sha(lock) == seal["lock_sha256"]
    entries = [
        p
        for p in tomllib.loads(lock.read_text())["package"]
        if p["name"] == "clipper2-rust"
    ]
    assert (
        len(entries) == 1
        and entries[0]["version"] == "1.1.0"
        and "source" not in entries[0]
    )
    return seal


def cases():
    result = []
    files = sorted((HERE / "corpus/t68sites").glob("*.corpus")) + sorted(
        (HERE / "corpus/t55").glob("*.cases")
    )
    assert len(files) == 8, files
    for path in files:
        text = path.read_text()
        blocks = re.findall(r"^case\s.*?^end\s*$", text, re.M | re.S)
        assert blocks, path
        for index, block in enumerate(blocks):
            result.append((path, index, block + "\n"))
    return result


def main():
    seal = seal_check()
    population = cases()
    RAW.mkdir(parents=True, exist_ok=True)
    manifest = {
        "seal_sha256": sha(SCRATCH / "seal.json"),
        "seal": seal,
        "corpus": {p.relative_to(HERE).as_posix(): sha(p) for p, _, _ in population},
        "case_count": len(population),
        "timeout_per_case_seconds": 180,
        "protocol": "fresh process per case; warmup then 5 timed repeats; forward Rust then C++; reverse C++ then Rust",
    }
    mp = RAW / "manifest.json"
    if mp.exists():
        assert json.loads(mp.read_text()) == manifest, "resume provenance changed"
    mp.write_text(json.dumps(manifest, indent=2) + "\n")
    binaries = {
        "rust": SCRATCH / "rust-harness/target/release/t69-dual-engine.exe",
        "cpp": SCRATCH / "cpp-driver.exe",
    }
    for order in ("forward", "reverse"):
        sequence = population if order == "forward" else list(reversed(population))
        engines = ("rust", "cpp") if order == "forward" else ("cpp", "rust")
        for path, index, block in sequence:
            directory = RAW / order / path.stem / str(index)
            directory.mkdir(parents=True, exist_ok=True)
            inp = directory / "input.case"
            inp.write_text(block)
            for engine in engines:
                out = directory / (engine + ".jsonl")
                if out.exists():
                    row = json.loads(out.read_text())
                    assert len(row["repeat_times_ns"]) == 5
                    continue
                with (directory / (engine + ".stderr.log")).open("w") as err:
                    run = subprocess.run(
                        [str(binaries[engine]), str(inp)],
                        capture_output=False,
                        stdout=subprocess.PIPE,
                        stderr=err,
                        timeout=180,
                        check=True,
                    )
                rows = run.stdout.decode().splitlines()
                assert len(rows) == 1
                row = json.loads(rows[0])
                assert (
                    len(row["repeat_times_ns"]) == 5
                    and min(row["repeat_times_ns"]) >= 0
                )  # Empty-input cases can hit Windows clock resolution; retain zeros.
                out.write_bytes(run.stdout)
            print(f"{order} {path.name} {index} complete", flush=True)
    seal_check()
    print("CAMPAIGN PASS", flush=True)


if __name__ == "__main__":
    main()
