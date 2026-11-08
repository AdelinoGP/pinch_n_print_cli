"""Idempotently build isolated, pinned engines; never mutate upstream sources."""

import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tarfile
import tomllib

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
SCRATCH = REPO / ".local-artifacts/perf-vs-orca/t69-clipper-engine"
HARNESS = SCRATCH / "rust-harness"


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inventory(path):
    return {
        p.relative_to(path).as_posix(): digest(p)
        for p in sorted(path.rglob("*"))
        if p.is_file()
    }


def main():
    SCRATCH.mkdir(parents=True, exist_ok=True)
    # The enclosing perf-vs-orca directory is not ignored in every checkout.
    # Keep all build/raw payloads (including this ignore file) local explicitly.
    (SCRATCH / ".gitignore").write_text("*\n")
    cache = list((Path.home() / ".cargo/registry/src").glob("*/clipper2-rust-1.1.0"))
    assert len(cache) == 1, cache
    upstream = cache[0]
    original = inventory(upstream)
    archive = (
        Path.home()
        / ".cargo/registry/cache"
        / upstream.parent.name
        / "clipper2-rust-1.1.0.crate"
    )
    locked = [
        p
        for p in tomllib.loads((REPO / "Cargo.lock").read_text())["package"]
        if p["name"] == "clipper2-rust" and p["version"] == "1.1.0"
    ]
    assert len(locked) == 1 and digest(archive) == locked[0]["checksum"], (
        "crate archive checksum mismatch"
    )
    # This cache has no .cargo-checksum.json. Compare every archived file directly
    # against the registry extraction, after verifying the archive's lock checksum.
    with tarfile.open(archive, "r:gz") as tar:
        for member in tar.getmembers():
            if member.isfile():
                relative = Path(member.name).relative_to("clipper2-rust-1.1.0")
                assert (upstream / relative).read_bytes() == tar.extractfile(
                    member
                ).read(), ("registry differs from archive", str(relative))
    copied = SCRATCH / "clipper2-rust-1.1.0"
    if not copied.exists():
        shutil.copytree(upstream, copied)
    assert inventory(copied) == original, (
        "scratch crate must be byte-identical, refusing to overwrite"
    )
    assert (
        tomllib.loads((copied / "Cargo.toml").read_text())["package"]["version"]
        == "1.1.0"
    )
    (HARNESS / "src").mkdir(parents=True, exist_ok=True)
    shutil.copy2(HERE / "driver.rs", HARNESS / "src/main.rs")
    (HARNESS / "Cargo.toml").write_text("""[package]
name = "t69-dual-engine"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
clipper2-rust = "=1.1.0"
serde_json = "1"
[patch.crates-io]
clipper2-rust = { path = "../clipper2-rust-1.1.0" }
""")
    cpp = REPO / "OrcaSlicerDocumented/deps_src/clipper2"
    assert "project(Clipper2 VERSION 1.5.2" in (cpp / "CMakeLists.txt").read_text()
    cpp_inventory = inventory(cpp)
    root_lock = digest(REPO / "Cargo.lock")
    commands = []
    environment = os.environ.copy()
    environment["CARGO_TARGET_DIR"] = str(HARNESS / "target")
    # Do not accidentally inherit experimental compiler flags from a parent run.
    for name in (
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
    ):
        environment.pop(name, None)

    def run(command, cwd, timeout=900):
        command = [str(c) for c in command]
        entry = {"cwd": str(cwd), "argv": command, "shell": shlex.join(command)}
        commands.append(entry)
        print(json.dumps(entry), flush=True)
        subprocess.run(command, cwd=cwd, env=environment, check=True, timeout=timeout)

    run(["cargo", "generate-lockfile", "--offline"], HARNESS)

    def seal_lock():
        lock = tomllib.loads((HARNESS / "Cargo.lock").read_text())
        entries = [p for p in lock["package"] if p["name"] == "clipper2-rust"]
        assert (
            len(entries) == 1
            and entries[0]["version"] == "1.1.0"
            and "source" not in entries[0]
        ), entries
        return entries[0]

    resolved = seal_lock()
    sources = [
        cpp / "Clipper2Lib/src" / name
        for name in (
            "clipper.engine.cpp",
            "clipper.offset.cpp",
            "clipper.rectclip.cpp",
            "clipper2_z.cpp",
        )
    ]
    run(
        [
            "g++",
            "-O3",
            "-std=c++17",
            "-static",
            "-static-libgcc",
            "-static-libstdc++",
            "-I",
            cpp / "Clipper2Lib/include",
            *sources,
            HERE / "driver.cpp",
            "-o",
            SCRATCH / "cpp-driver.exe",
        ],
        SCRATCH,
    )
    run(["cargo", "build", "--release", "--locked", "--offline"], HARNESS)
    assert resolved == seal_lock()
    assert inventory(upstream) == original, "registry changed"
    assert inventory(copied) == original, "scratch crate changed"
    assert inventory(cpp) == cpp_inventory, "canonical C++ source changed"
    assert digest(REPO / "Cargo.lock") == root_lock, "root lock changed during setup"
    provenance = {
        "rustc": subprocess.check_output(["rustc", "-vV"], text=True, timeout=30),
        "g++": subprocess.check_output(["g++", "--version"], text=True, timeout=30),
        "commands": commands,
        "registry_path": str(upstream),
        "crate_package_checksum": digest(archive),
        "registry_matches_verified_archive": True,
        "crate_inventory": original,
        "crate_inventory_sha256": hashlib.sha256(
            json.dumps(original, sort_keys=True).encode()
        ).hexdigest(),
        "cpp_version": "1.5.2",
        "cpp_inventory": cpp_inventory,
        "resolved_clipper": resolved,
        "lock_sha256": digest(HARNESS / "Cargo.lock"),
        "root_lock_unchanged": True,
        "registry_unchanged": True,
        "pristine_copy_byte_identical": True,
        "lock_seal_passed": True,
        "driver_sources": {
            p.name: digest(p) for p in (HERE / "driver.rs", HERE / "driver.cpp")
        },
        "environment": {
            "CARGO_TARGET_DIR": environment["CARGO_TARGET_DIR"],
            "compiler_flag_overrides_removed": True,
        },
        "binaries": {
            str(p.relative_to(SCRATCH)): digest(p)
            for p in (
                SCRATCH / "cpp-driver.exe",
                HARNESS / "target/release/t69-dual-engine.exe",
            )
        },
    }
    (SCRATCH / "seal.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print("PROVENANCE " + json.dumps(provenance, indent=2), flush=True)


if __name__ == "__main__":
    main()
