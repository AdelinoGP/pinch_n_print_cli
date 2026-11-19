"""Build all four t167 replay engines cleanly with full provenance sealing."""

import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
ORCA_PIN = REPO.parent / "Orca(pnp_gui)"
ORCA_DOC = REPO / "OrcaSlicerDocumented"
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t167-canonical-clipperutils-replay"
HARNESS = SCRATCH / "rust-harness"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_rev(repo: Path) -> str:
    return subprocess.check_output(
        ["git", "-C", str(repo), "rev-parse", "HEAD"], text=True, timeout=30
    ).strip()


def run(cmd, cwd, env, commands, timeout=900):
    argv = [str(c) for c in cmd]
    entry = {"cwd": str(cwd), "argv": argv, "shell": shlex.join(argv)}
    commands.append(entry)
    print(json.dumps(entry), flush=True)
    subprocess.run(argv, cwd=cwd, env=env, check=True, timeout=timeout)


def main():
    SCRATCH.mkdir(parents=True, exist_ok=True)
    (SCRATCH / ".gitignore").write_text("*\n")

    assert ORCA_PIN.is_dir(), f"Missing canonical v2.5.0 checkout: {ORCA_PIN}"
    assert ORCA_DOC.is_dir(), f"Missing OrcaSlicerDocumented checkout: {ORCA_DOC}"
    orca_pin_rev = git_rev(ORCA_PIN)
    orca_doc_rev = git_rev(ORCA_DOC)
    pnp_rev = git_rev(REPO)

    c2_152_dir = ORCA_DOC / "deps_src/clipper2"
    c2_201_dir = ORCA_PIN / "deps_src/clipper2"
    c1_dir = ORCA_DOC / "deps_src/clipper"
    assert "VERSION 1.5.2" in (c2_152_dir / "CMakeLists.txt").read_text()
    assert "VERSION 2.0.1" in (c2_201_dir / "CMakeLists.txt").read_text()

    env = os.environ.copy()
    for k in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"):
        env.pop(k, None)

    commands = []
    common_cxx = ["g++", "-O3", "-std=c++17", "-static", "-static-libgcc", "-static-libstdc++"]

    # 1) C++ Clipper2 1.5.2 (t69 continuity twin)
    c2_152_srcs = [
        c2_152_dir / "Clipper2Lib/src" / n
        for n in ("clipper.engine.cpp", "clipper.offset.cpp", "clipper.rectclip.cpp", "clipper2_z.cpp")
    ]
    bin_c2_152 = SCRATCH / "driver-cpp-c2-152.exe"
    run(
        [
            *common_cxx,
            "-I", c2_152_dir / "Clipper2Lib/include",
            *c2_152_srcs,
            HERE / "driver_cpp_c2.cpp",
            "-o", bin_c2_152,
        ],
        SCRATCH, env, commands,
    )

    # 2) Canonical OrcaSlicer v2.5.0 ClipperUtils on Clipper2 2.0.1
    c2_201_srcs = [
        c2_201_dir / "Clipper2Lib/src" / n
        for n in ("clipper.engine.cpp", "clipper.offset.cpp", "clipper.rectclip.cpp", "clipper2_z.cpp")
    ]
    bin_orca_v250 = SCRATCH / "driver-orca-v250.exe"
    run(
        [
            *common_cxx,
            "-I", HERE / "shims",
            "-I", c2_201_dir / "Clipper2Lib/include",
            *c2_201_srcs,
            HERE / "driver_orca_v250.cpp",
            "-o", bin_orca_v250,
        ],
        SCRATCH, env, commands,
    )

    # 3) Legacy pre-2.5.0 OrcaSlicer Clipper1 (ClipperLib 6.2.6)
    bin_orca_c1 = SCRATCH / "driver-orca-c1.exe"
    run(
        [
            *common_cxx,
            "-I", HERE / "shims",
            "-I", ORCA_DOC / "src",
            "-I", c1_dir,
            c1_dir / "clipper.cpp",
            HERE / "driver_orca_c1.cpp",
            "-o", bin_orca_c1,
        ],
        SCRATCH, env, commands,
    )

    # 4) PnP current Rust Clipper2 (in-tree vendor/clipper2-rust + mimalloc)
    (HARNESS / "src").mkdir(parents=True, exist_ok=True)
    shutil.copy2(HERE / "driver_rust.rs", HARNESS / "src/main.rs")
    vendor_clipper = (REPO / "vendor/clipper2-rust").as_posix()
    (HARNESS / "Cargo.toml").write_text(f"""[package]
name = "t167-rust-driver"
version = "0.0.0"
edition = "2021"

[workspace]

[dependencies]
clipper2-rust = {{ path = "{vendor_clipper}" }}
mimalloc = "0.1"
serde_json = "1"
""")
    rust_env = env.copy()
    rust_env["CARGO_TARGET_DIR"] = str(HARNESS / "target")
    run(["cargo", "build", "--release", "--offline"], HARNESS, rust_env, commands)
    bin_rust = HARNESS / "target/release/t167-rust-driver.exe"
    assert bin_rust.is_file(), f"Missing Rust binary: {bin_rust}"

    provenance = {
        "pnp_git_rev": pnp_rev,
        "orca_v250_git_rev": orca_pin_rev,
        "orca_v250_path": str(ORCA_PIN),
        "orca_documented_git_rev": orca_doc_rev,
        "orca_documented_path": str(ORCA_DOC),
        "rustc": subprocess.check_output(["rustc", "-vV"], text=True, timeout=30).strip(),
        "gxx": subprocess.check_output(["g++", "--version"], text=True, timeout=30).strip(),
        "cxx_flags": common_cxx,
        "allocator_notes": {
            "rust_pnp": "mimalloc::MiMalloc (matches pnp_cli global allocator)",
            "cpp_c2_1_5_2": "MinGW UCRT/MSVCRT std::malloc (matches t69)",
            "orca_v2_5_0": "MinGW std::malloc via shims/oneapi/tbb/scalable_allocator.h (Orca production uses tbbmalloc)",
            "orca_clipper1": "MinGW std::allocator via shims/oneapi/tbb/scalable_allocator.h (Orca production used tbbmalloc)",
        },
        "commands": commands,
        "driver_sources": {
            n: sha256(HERE / n)
            for n in (
                "driver_rust.rs",
                "driver_cpp_c2.cpp",
                "driver_orca_v250.cpp",
                "driver_orca_c1.cpp",
                "shims/oneapi/tbb/scalable_allocator.h",
                "shims/Eigen/Geometry",
            )
        },
        "binaries": {
            "rust_pnp": {"path": str(bin_rust), "sha256": sha256(bin_rust)},
            "cpp_c2_1_5_2": {"path": str(bin_c2_152), "sha256": sha256(bin_c2_152)},
            "orca_v2_5_0": {"path": str(bin_orca_v250), "sha256": sha256(bin_orca_v250)},
            "orca_clipper1": {"path": str(bin_orca_c1), "sha256": sha256(bin_orca_c1)},
        },
    }
    (SCRATCH / "provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print("SETUP PASS", flush=True)


if __name__ == "__main__":
    main()
