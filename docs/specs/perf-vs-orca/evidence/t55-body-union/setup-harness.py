"""Create a private pinned diagnostic tree; never modify registry or production code."""

import difflib
import json
from pathlib import Path
import shutil
import subprocess
import tomllib

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[4]
RAW = REPO / ".local-artifacts/perimeter-reference-preparation/t55-body-union"
H = RAW / "harness"


def setup():
    if H.exists():
        raise RuntimeError("one-shot harness setup guard")
    H.mkdir()
    (H / "src").mkdir()
    clipper = list((Path.home() / ".cargo/registry/src").glob("*/clipper2-rust-1.1.0"))
    assert len(clipper) == 1
    shutil.copytree(clipper[0], H / "clipper2-rust")
    shutil.copy2(REPO / "Cargo.lock", H / "Cargo.lock")
    (H / "tests/fixtures").mkdir(parents=True)
    shutil.copy2(
        REPO
        / "crates/slicer-core/tests/fixtures/clip_polylines_prepared_reference.json",
        H / "tests/fixtures/clip_polylines_prepared_reference.json",
    )
    shutil.copy2(HERE / "harness.rs", H / "src/main.rs")
    shutil.copy2(
        REPO / "crates/slicer-core/src/polygon_ops.rs",
        H / "src/measured_polygon_ops.rs",
    )
    manifest = """[package]
name = "t55-native-diagnostic"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
tree-support-planner = { path = "TREE" }
slicer-core = { path = "CORE", features = ["host-algos"] }
slicer-ir = { path = "IR" }
clipper2-rust = "=1.1.0"
serde_json = "1"
[patch.crates-io]
clipper2-rust = { path = "clipper2-rust" }
"""
    for k, v in [
        ("TREE", "modules/core-modules/tree-support-planner"),
        ("CORE", "crates/slicer-core"),
        ("IR", "crates/slicer-ir"),
    ]:
        manifest = manifest.replace(k, (REPO / v).as_posix())
    (H / "Cargo.toml").write_text(manifest)
    provenance = {
        "clipper_original": str(clipper[0]),
        "production_source": "crates/slicer-core/src/polygon_ops.rs",
        "compiler": subprocess.check_output(["rustc", "-vV"], text=True),
        "root_lock_sha256": digest(REPO / "Cargo.lock"),
        "clipper_original_inventory": inventory(clipper[0]),
        "root_packages": tomllib.loads((REPO / "Cargo.lock").read_text())["package"],
    }
    (HERE / "harness-provenance.json").write_text(
        json.dumps(provenance, indent=2) + "\n"
    )


def digest(p):
    import hashlib

    with p.open("rb") as f:
        return hashlib.file_digest(f, "sha256").hexdigest()


def inventory(p):
    return {q.relative_to(p).as_posix(): digest(q) for q in p.rglob("*") if q.is_file()}


def seal():
    before = json.loads((HERE / "harness-provenance.json").read_text())
    assert (
        inventory(Path(before["clipper_original"]))
        == before["clipper_original_inventory"]
    ), "registry changed"
    pairs = [
        (
            Path(before["clipper_original"]) / "src/engine_public.rs",
            H / "clipper2-rust/src/engine_public.rs",
        ),
        (
            Path(before["clipper_original"]) / "src/engine.rs",
            H / "clipper2-rust/src/engine.rs",
        ),
        (
            Path(before["clipper_original"]) / "src/lib.rs",
            H / "clipper2-rust/src/lib.rs",
        ),
        (
            REPO / "crates/slicer-core/src/polygon_ops.rs",
            H / "src/measured_polygon_ops.rs",
        ),
    ]
    patch = ""
    for original, probe in pairs:
        old = original.read_bytes().decode()
        new = probe.read_bytes().decode()
        rel = probe.relative_to(H).as_posix()
        patch += "".join(
            difflib.unified_diff(
                old.splitlines(True),
                new.splitlines(True),
                fromfile="a/" + rel,
                tofile="b/" + rel,
            )
        )
    added = H / "clipper2-rust/src/t55_probe.rs"
    patch += "".join(
        difflib.unified_diff(
            [],
            added.read_text().splitlines(True),
            fromfile="/dev/null",
            tofile="b/clipper2-rust/src/t55_probe.rs",
        )
    )
    (HERE / "internals.patch").write_bytes(patch.encode())
    lock = tomllib.loads((H / "Cargo.lock").read_text())
    cp = [p for p in lock["package"] if p["name"] == "clipper2-rust"]
    assert len(cp) == 1 and cp[0]["version"] == "1.1.0" and "source" not in cp[0]
    roots = {(p["name"], p["version"]) for p in before["root_packages"]}
    extras = [
        (p["name"], p["version"])
        for p in lock["package"]
        if (p["name"], p["version"]) not in roots
        and p["name"] != "t55-native-diagnostic"
    ]
    assert not extras, extras
    before["resolved_clipper"] = cp
    before["lock_sha256"] = digest(H / "Cargo.lock")
    before["harness_sha256"] = digest(H / "src/main.rs")
    assert digest(HERE / "harness.rs") == before["harness_sha256"]
    before["diagnostic_sources"] = {
        p.relative_to(H).as_posix(): digest(p)
        for p in [*(probe for _, probe in pairs), added]
    }
    check = RAW / "patch-check"
    assert not check.exists(), "one-shot patch reproduction guard"
    check.mkdir()
    for original, probe in pairs:
        target = check / probe.relative_to(H)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(original, target)
    subprocess.run(
        ["git", "apply", "--check", str(HERE / "internals.patch")],
        cwd=check,
        check=True,
    )
    subprocess.run(
        ["git", "apply", str(HERE / "internals.patch")], cwd=check, check=True
    )
    assert all(digest(check / p) == h for p, h in before["diagnostic_sources"].items())
    before["internals_patch_reapplied_exactly"] = True
    before["no_new_dependency_versions"] = True
    (HERE / "harness-provenance.json").write_text(json.dumps(before, indent=2) + "\n")
    shutil.copy2(H / "Cargo.lock", HERE / "harness.Cargo.lock")


if __name__ == "__main__":
    import sys

    {"setup": setup, "seal": seal}[sys.argv[1]]()
