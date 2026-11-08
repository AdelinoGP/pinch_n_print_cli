"""Build independent native/WASM check harnesses without touching production source."""

import shutil
from pathlib import Path
import manage

raw = manage.RAW
source = manage.SOURCE
oracle = source / ".local-libs/clipper2-original"
assert not oracle.exists(), "one-shot oracle setup"
registry = list((Path.home() / ".cargo/registry/src").glob("*/clipper2-rust-1.1.0"))
assert len(registry) == 1
shutil.copytree(registry[0], oracle)
manifest = (oracle / "Cargo.toml").read_text()
assert manifest.count('name = "clipper2-rust"') == 2
manifest = manifest.replace(
    'name = "clipper2-rust"', 'name = "clipper2-rust-oracle"', 1
)
(oracle / "Cargo.toml").write_text(manifest + "\n[workspace]\n")
assert manage.inv(oracle / "src") == manage.inv(registry[0] / "src"), (
    "oracle algorithms changed"
)
harness = raw / "checks"
(harness / "src").mkdir(parents=True)
shutil.copy2(manage.HERE / "checks.rs", harness / "src/lib.rs")
(harness / "Cargo.toml").write_text(f'''[package]
name = "t57-engine-checks"
version = "0.0.0"
edition = "2021"
[workspace]
[lib]
crate-type = ["rlib", "cdylib"]
[dependencies]
clipper_candidate = {{ package = "clipper2-rust", path = "{manage.LIB.as_posix()}", version = "=1.1.0", features = ["topx-test-support"] }}
clipper_original = {{ package = "clipper2-rust-oracle", path = "{oracle.as_posix()}", version = "=1.1.0" }}
''')
shutil.copy2(manage.REPO / "Cargo.lock", harness / "Cargo.lock")
print("PASS: independent pristine oracle and shared native/WASM checks ready")
