"""t70 candidate measurement setup: pristine vs candidate harnesses.

Builds two isolated harness workspaces over the SAME tier-1 driver
(profile-driver.rs):
  - pristine-harness: depends on the t69 sealed pristine scratch copy
    (.local-artifacts/perf-vs-orca/t69-clipper-engine/clipper2-rust-1.1.0,
    byte-verified against the registry archive by t69's setup-engines.py).
  - cand-harness: depends on the CURRENT vendored tree (vendor/clipper2-rust),
    i.e. the candidate source of truth.

Run both drivers over the corpus and hand the JSONL pair to
candidate-reduce.py (per-case hash identity + paired medians).
"""

import hashlib
import json
import os
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
# This script sits in the t70 evidence root (not profiling/): parents[0]=evidence,
# [1]=perf-vs-orca, [2]=specs, [3]=docs, [4]=repo root.
REPO = HERE.parents[4]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t70-clipper-candidate"
VENDOR = REPO / "vendor/clipper2-rust"
PRISTINE = REPO / ".local-artifacts/perf-vs-orca/t69-clipper-engine/clipper2-rust-1.1.0"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_harness(name: str, dep_dir: Path) -> Path:
    harness = SCRATCH / name
    (harness / "src").mkdir(parents=True, exist_ok=True)
    (harness / "src/main.rs").write_bytes(
        (HERE / "profiling/profile-driver.rs").read_bytes()
    )
    rel = os.path.relpath(dep_dir, harness).replace("\\", "/")
    (harness / "Cargo.toml").write_text(
        f"""[package]
name = "t70-{name}"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
clipper2-rust = {{ path = "{rel}" }}
serde_json = "1"
[profile.release]
debug = false
codegen-units = 1
"""
    )
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(harness / "target")
    for n in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER"):
        env.pop(n, None)
    subprocess.run(
        ["cargo", "build", "--release", "--offline"],
        cwd=harness,
        env=env,
        check=True,
        timeout=900,
    )
    exe = harness / "target/release" / f"t70-{name}.exe"
    print(f"[t70] {name} built:", exe, "sha256:", digest(exe))
    return exe


def main() -> int:
    provenance = json.loads((HERE / "vendor-provenance.json").read_text())
    # The candidate lives in the vendor tree now; the pristine inventory must
    # NOT match anymore — record the current (candidate) inventory separately.
    current = {
        p.relative_to(VENDOR).as_posix(): digest(p)
        for p in sorted(VENDOR.rglob("*"))
        if p.is_file()
    }
    (HERE / "candidate-inventory.json").write_text(
        json.dumps(current, indent=2, sort_keys=True) + "\n"
    )
    assert PRISTINE.is_dir(), "t69 pristine scratch copy missing"
    assert (
        digest(PRISTINE / "Cargo.toml") == provenance["vendor_inventory"]["Cargo.toml"]
    ), "pristine scratch copy drifted"
    print("[t70] pristine scratch copy verified against provenance Cargo.toml hash")
    build_harness("pristine-harness", PRISTINE)
    build_harness("cand-harness", VENDOR)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
