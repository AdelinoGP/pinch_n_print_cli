"""Decisive provenance experiment for the ticket-19 candidate source form.

For each candidate source form: is the tree clippy-clean under `-D warnings`,
and does a forced rebuild reproduce the *frozen (timed)* guest artifacts
byte-for-byte? Uses artifact deletion + explicit rebuild confirmation so a
stale artifact can never masquerade as a reproduction.

Ends with the measured source restored and both artifacts rebuilt.
"""

import hashlib
import subprocess
from pathlib import Path

SRC = Path("crates/slicer-core/src/polygon_ops.rs")
ROOT = Path("Cargo.toml")
ACCEL_GUEST_LINE = "classic-perimeters-guest"
ORD = Path("modules/core-modules/classic-perimeters/classic-perimeters.wasm")
ACCEL = Path(
    "target/guests-accelerated/artifacts/modules/core-modules/classic-perimeters/classic-perimeters.wasm"
)

MEASURED = "44495e253631366c068723f360e414fe86639dc59bf22b8682409972d66514d1"
TIMED_ORD = "f67d08562b9a3d65592554ebe2cc0c0247736c8bd425dbb1d855a159cf812c53"
TIMED_ACC = "332c41919023e4c5dc67fb5bf30ab86a508875237bf2b45dac7c5269b977b96b"

OLD = b"""    if !(delta_mm > 0.0) {"""
ALLOW = b"""#[allow(clippy::neg_cmp_op_on_partial_ord)] // deliberate NaN-inclusive test
"""
POS = b"""    if delta_mm > 0.0 {"""


def sha(p):
    return hashlib.sha256(Path(p).read_bytes()).hexdigest()


def build(accel):
    artifact = ACCEL if accel else ORD
    artifact.unlink(missing_ok=True)
    args = ["cargo", "xtask", "build-guests"] + (["--accelerated"] if accel else [])
    r = subprocess.run(args, capture_output=True, text=True)
    if r.returncode != 0 or ACCEL_GUEST_LINE not in r.stdout:
        return f"FAILED/not-rebuilt(rc={r.returncode})"
    return sha(artifact)


def narrow_clippy():
    r = subprocess.run(
        [
            "cargo",
            "clippy",
            "-p",
            "slicer-core",
            "-p",
            "classic-perimeters",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        capture_output=True,
        text=True,
    )
    return r.returncode == 0


def variant(tag, mutate_src=None, mutate_root=None):
    src_before = sha(SRC)
    root_before = sha(ROOT)
    if mutate_src is not None:
        SRC.write_bytes(mutate_src(SRC.read_bytes()))
    if mutate_root is not None:
        ROOT.write_bytes(mutate_root(ROOT.read_bytes()))
    ok = narrow_clippy()
    ord_h = build(False)
    acc_h = build(True)
    print(
        f"{tag:>3} src={src_before[:12]} root={root_before[:12]} clippy={ok!s:>5} "
        f"ord={ord_h[:16]}{'==TIMED' if ord_h == TIMED_ORD else '':>8} "
        f"accel={acc_h[:16]}{'==TIMED' if acc_h == TIMED_ACC else '':>8}",
        flush=True,
    )
    return ok, ord_h, acc_h


assert sha(SRC) == MEASURED, f"expected measured source on disk, got {sha(SRC)}"
measured_bytes = SRC.read_bytes()

# V0: measured source, no mutation (the timed source; clippy is expected to fail).
variant("V0")

# V1: measured source + root workspace lint allow (no source text change).
root_orig = ROOT.read_bytes()
anchor = b"[workspace.lints.clippy]\n"
assert anchor in root_orig
root_v1 = root_orig.replace(
    anchor,
    anchor + b"# `!(d > 0.0)` is the deliberate NaN-inclusive non-positive test; the\n"
    b"# positive-first rewrite recompiles the wasm and would orphan the timed guest.\n"
    b'neg_cmp_op_on_partial_ord = "allow"\n',
    1,
)
variant("V1", mutate_root=lambda _: root_v1)
ROOT.write_bytes(root_orig)

# V2: measured source + per-function allow (source text change).
variant(
    "V2",
    mutate_src=lambda b: b.replace(
        b"pub fn opening_is_empty(", ALLOW + b"pub fn opening_is_empty(", 1
    ),
)

# V3: positive-first control-flow rewrite (source text change).
variant("V3", mutate_src=lambda b: b.replace(OLD, POS, 1))

# Restore the measured source and both artifacts.
print("restoring measured source", flush=True)
SRC.write_bytes(measured_bytes)
assert sha(SRC) == MEASURED, sha(SRC)
print(f"restored src={sha(SRC)[:12]} clippy={narrow_clippy()}", flush=True)
o = build(False)
a = build(True)
print(f"restored: ord={o[:16]} accel={a[:16]}", flush=True)
print(f"final: ord==timed {o == TIMED_ORD}; accel==timed {a == TIMED_ACC}", flush=True)
print(
    f"root manifest unchanged: {sha(ROOT) == hashlib.sha256(root_orig).hexdigest()}",
    flush=True,
)
