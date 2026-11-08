"""Sabotage separate copies, never the live candidate; require native and WASM rejection."""

import json
import shutil
import subprocess
import sys
import manage as m

result = {}
resume = sys.argv[1:] == ["--resume-after-workspace-fix"]
assert not sys.argv[1:] or resume
for name, old, new in [
    ("stale_y", "&& self.y == y", "&& true"),
    ("wrong_projection", "            x,\n", "            x: x + 1,\n"),
]:
    root = m.RAW / "negative-controls" / name
    existing = resume and name == "stale_y"
    assert root.exists() if existing else not root.exists(), (
        "one-shot negative-control guard"
    )
    lib = root / "engine"
    if not existing:
        shutil.copytree(m.LIB, lib, ignore=shutil.ignore_patterns("target"))
    engine = lib / "src/engine.rs"
    text = engine.read_text()
    if not existing:
        assert text.count(old) == 1, "unambiguous sabotage seam"
        engine.write_text(text.replace(old, new))
        (root / "src").mkdir()
    else:
        assert old not in text and new in text, "sabotage drift"
    shutil.copy2(m.HERE / "checks.rs", root / "src/lib.rs")
    (root / "Cargo.toml").write_text(f'''[package]
name = "t57-negative-{name.replace("_", "-")}"
version = "0.0.0"
edition = "2021"
[workspace]
exclude = ["engine"]
[lib]
crate-type = ["rlib", "cdylib"]
[dependencies]
clipper_candidate = {{ package = "clipper2-rust", path = "engine", version = "=1.1.0", features = ["topx-test-support"] }}
clipper_original = {{ package = "clipper2-rust-oracle", path = "{m.SOURCE.as_posix()}/.local-libs/clipper2-original", version = "=1.1.0" }}
''')
    shutil.copy2(m.REPO / "Cargo.lock", root / "Cargo.lock")
    cmd = [
        "cargo",
        "test",
        "--manifest-path",
        str(root / "Cargo.toml"),
        "--offline",
        "--release",
        "--lib",
        "--",
        "native_shared_engine_cache_and_ordered_geometry_contract",
        "--nocapture",
    ]
    log = (
        m.HERE
        / "logs"
        / (
            name
            + (
                "-native-negative-workspace-resolved.log"
                if existing
                else "-native-negative.log"
            )
        )
    )
    with (m.REPO / "target/test-output.log").open("wb") as tee, log.open("wb") as saved:
        p = subprocess.Popen(
            cmd, cwd=m.REPO, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
        )
        for line in p.stdout:
            tee.write(line)
            saved.write(line)
        native_exit = p.wait()
    text = log.read_text()
    assert native_exit == 101 and "1 failed" in text and "panicked at" in text, (
        "must reject at behavioral assertion, not compiler"
    )
    m.run(
        [
            "cargo",
            "build",
            "--manifest-path",
            root / "Cargo.toml",
            "--offline",
            "--release",
            "--lib",
            "--target",
            "wasm32-unknown-unknown",
        ],
        name + "-wasm-negative-build",
    )
    wasm = root / (
        "target/wasm32-unknown-unknown/release/t57_negative_" + name + ".wasm"
    )
    p = subprocess.run(
        ["node", str(m.HERE / "run-wasm.cjs"), str(wasm)], capture_output=True
    )
    (m.HERE / "logs" / (name + "-wasm-negative.log")).write_bytes(p.stdout + p.stderr)
    assert p.returncode != 0 and b"RuntimeError" in p.stderr, (
        "must trap executing actual WASM assertion"
    )
    result[name] = {
        "native_exit": native_exit,
        "native_assertion_rejected": True,
        "wasm_exit": p.returncode,
        "wasm_assertion_rejected": True,
        "sabotage": {"old": old, "new": new},
        "source_sha256": m.sha(engine),
    }
    m.write(m.HERE / "negative-controls.json", result)
    print("PASS: native and actual WASM rejected " + name, flush=True)
