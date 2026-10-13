"""Independent production-wrapper/mesh oracle; algorithms remain byte-identical."""

import shutil
import manage as m

oracle = m.SOURCE / ".local-libs/slicer-core-original"
assert not oracle.exists(), "one-shot consumer oracle setup"
shutil.copytree(m.SOURCE / "crates/slicer-core/src", oracle / "src")
shutil.copy2(m.REPO / "crates/slicer-core/build.rs", oracle / "build.rs")
assert m.inv(oracle / "src") == m.inv(m.REPO / "crates/slicer-core/src")
manifest = (m.REPO / "crates/slicer-core/Cargo.toml").read_text()
manifest = manifest.split("[dev-dependencies]")[0]
manifest = manifest.replace('name = "slicer-core"', 'name = "slicer-core-oracle"', 1)
manifest = manifest.replace('path = "../', 'path = "../../crates/')
manifest = manifest.replace(
    "clipper2-rust = { workspace = true }",
    'clipper2-rust = { package = "clipper2-rust-oracle", path = "../clipper2-original", version = "=1.1.0" }',
)
(oracle / "Cargo.toml").write_text(manifest + "\n[workspace]\n")
harness = m.RAW / "consumers"
(harness / "src").mkdir(parents=True)
shutil.copy2(m.HERE / "consumers.rs", harness / "src/lib.rs")
(harness / "Cargo.toml").write_text(f'''[package]
name = "t57-consumer-checks"
version = "0.0.0"
edition = "2021"
[workspace]
[lib]
crate-type = ["rlib", "cdylib"]
[dependencies]
core_candidate = {{ package = "slicer-core", path = "{m.SOURCE.as_posix()}/crates/slicer-core" }}
core_original = {{ package = "slicer-core-oracle", path = "{oracle.as_posix()}" }}
slicer-ir = {{ path = "{m.SOURCE.as_posix()}/crates/slicer-ir" }}
''')
shutil.copy2(m.REPO / "Cargo.lock", harness / "Cargo.lock")
m.write(
    m.HERE / "consumer-oracle.json",
    {
        "production_algorithm_inventory": m.inv(oracle / "src"),
        "oracle_manifest": (oracle / "Cargo.toml").read_text(),
        "difference": "package/dependency identity only; unchanged production algorithms with pristine independent engine",
    },
)
print("PASS: independent production mesh/boolean/offset wrappers prepared")
