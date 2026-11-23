"""t70 tier-2 harness setup: build the instrumented-twin measurement workspace.

- Re-creates .local-artifacts/perf-vs-orca/t70-clipper-candidate/prof-twin from
  vendor/clipper2-rust by applying profiling/prof-twin.patch (git apply).
- Builds prof2-harness (isolated workspace, own target dir) whose driver
  depends on the instrumented twin by path.
- Verifies the vendor tree is still the sealed pristine copy first.
"""

import hashlib
import json
import os
import subprocess
from pathlib import Path
import tomllib

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[5]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t70-clipper-candidate"
TWIN = SCRATCH / "prof-twin"
HARNESS = SCRATCH / "prof2-harness"
VENDOR = REPO / "vendor" / "clipper2-rust"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    provenance = json.loads((HERE.parent / "vendor-provenance.json").read_text())
    current = {
        p.relative_to(VENDOR).as_posix(): digest(p)
        for p in sorted(VENDOR.rglob("*"))
        if p.is_file()
    }
    assert current == provenance["vendor_inventory"], (
        "vendor tree drifted from provenance"
    )

    if TWIN.exists():
        subprocess.run(["rm", "-rf", str(TWIN)], check=True)
    subprocess.run(["cp", "-r", str(VENDOR), str(TWIN)], check=True)
    patch = (HERE / "prof-twin.patch").resolve()
    result = subprocess.run(
        [
            "git",
            "apply",
            "--directory=" + str(TWIN.relative_to(REPO).as_posix()),
            str(patch),
        ],
        cwd=REPO,
        check=False,
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, (
        f"patch application failed: {result.stdout} {result.stderr}"
    )
    assert (TWIN / "src/prof.rs").is_file(), "twin missing prof.rs"

    (HARNESS / "src").mkdir(parents=True, exist_ok=True)
    (HARNESS / "src/main.rs").write_bytes((HERE / "profile-driver2.rs").read_bytes())
    rel = os.path.relpath(TWIN, HARNESS).replace("\\", "/")
    (HARNESS / "Cargo.toml").write_text(
        f"""[package]
name = "t70-prof2-driver"
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
        tomllib.loads((TWIN / "Cargo.toml").read_text())["package"]["version"]
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
    exe = HARNESS / "target/release/t70-prof2-driver.exe"
    print("[t70] tier-2 harness built:", exe)
    print("[t70] binary sha256:", digest(exe))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
