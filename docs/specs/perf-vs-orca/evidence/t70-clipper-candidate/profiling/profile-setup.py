"""t70 profile-harness setup: build the scratch measurement workspace.

Creates .local-artifacts/perf-vs-orca/t70-clipper-candidate/prof-harness/, an
isolated cargo workspace whose driver depends on the IN-TREE vendored engine
(vendor/clipper2-rust) by path — the exact source production compiles. The
registry is never touched; the vendor tree is read-only here and stays
pristine (byte-identity re-verified against the registry extraction and the
t70 provenance inventory).

Idempotent; refuses an existing modified driver copy.
"""

import hashlib
import json
import os
import subprocess
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent
# profiling/ is one level deeper than the t70 evidence root the t69 scripts
# sit at: parents[0]=t70-clipper-candidate, [1]=evidence, [2]=perf-vs-orca,
# [3]=specs, [4]=docs, [5]=repo root.
REPO = HERE.parents[5]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t70-clipper-candidate"
HARNESS = SCRATCH / "prof-harness"
VENDOR = REPO / "vendor" / "clipper2-rust"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    provenance = json.loads((HERE.parent / "vendor-provenance.json").read_text())
    # The vendor tree must still be the sealed pristine copy.
    current = {
        p.relative_to(VENDOR).as_posix(): digest(p)
        for p in sorted(VENDOR.rglob("*"))
        if p.is_file()
    }
    assert current == provenance["vendor_inventory"], (
        "vendor tree drifted from provenance"
    )
    print("[t70] vendor tree still byte-identical to registry extraction")

    (HARNESS / "src").mkdir(parents=True, exist_ok=True)
    driver_src = HERE / "profile-driver.rs"
    driver_dst = HARNESS / "src/main.rs"
    if driver_dst.exists():
        assert digest(driver_dst) == digest(driver_src), (
            "existing scratch driver differs"
        )
    else:
        driver_dst.write_bytes(driver_src.read_bytes())
    rel = os.path.relpath(VENDOR, HARNESS).replace("\\", "/")
    (HARNESS / "Cargo.toml").write_text(
        f"""[package]
name = "t70-prof-driver"
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
    assert (
        tomllib.loads((VENDOR / "Cargo.toml").read_text())["package"]["version"]
        == "1.1.0"
    )
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(HARNESS / "target")
    for name in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER"):
        env.pop(name, None)
    subprocess.run(
        ["cargo", "build", "--release", "--offline"],
        cwd=HARNESS,
        env=env,
        check=True,
        timeout=900,
    )
    exe = HARNESS / "target/release/t70-prof-driver.exe"
    print("[t70] harness built:", exe)
    print("[t70] binary sha256:", digest(exe))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
