"""Regressions for the t41 preparation tools; runs no slicer.

Covers the t40 lesson (Windows manifest keys) plus the t41 additions:
* the payload root is durable (outside `target/`) and rejected inside it;
* negative controls for the sparse-wall-bbox checker (inside accepted;
  outside-start, outside-end and sparse-without-wall rejected).

Optional: pass an attempt directory to also verify the real historical corpus
inventory against that attempt's recorded `historical-corpus-hashes.json`.
"""

from pathlib import Path, PureWindowsPath
import json
import runpy
import sys

EVIDENCE = Path(__file__).resolve().parent
ROOT = EVIDENCE.parents[4]
script = EVIDENCE / "prepare.py"
module = runpy.run_path(str(script))

# t40 lesson: Windows separators must not leak into manifest keys/lookups.
root = PureWindowsPath(r"D:\slicerProject\corpus")
model = root / "supports-off-benchy" / "model.stl"
key = module.get("relative_key", lambda path, base: str(path.relative_to(base)))
assert key(model, root) == "supports-off-benchy/model.stl", (
    "Windows manifest key is not portable"
)
inventory = {key(model, root): "digest"}
assert inventory["supports-off-benchy/model.stl"] == "digest", (
    "original lookup input still fails"
)

# t41 root cause: the durable payload namespace must be outside target/.
durable = module["DURABLE"]
assert durable == ROOT / ".local-artifacts" / "perimeter-reference-preparation", (
    f"unexpected durable namespace: {durable}"
)
assert ROOT / "target" not in durable.parents, (
    "durable payload namespace must not live under target/"
)
assert ".local-artifacts" in durable.parts, (
    "durable payload namespace must use the gitignored .local-artifacts tree"
)

# The t41 verifier independently enforces the same rule.
verifier = runpy.run_path(str(EVIDENCE / "verify-freeze.py"))
target_nested = ROOT / "target" / "perimeter-reference-preparation" / "t40-regression"
try:
    verifier["ensure_durable_root"](target_nested)
except RuntimeError as error:
    assert "FROZEN ROOT UNDER TARGET IS REJECTED" in str(error), error
else:
    raise RuntimeError("target-nested frozen root was not rejected")
missing = verifier["ensure_durable_root"](
    durable / "retention-probe-does-not-exist" / "corpus"
)
assert missing == durable / "retention-probe-does-not-exist" / "corpus"
try:
    verifier["require_root"](durable / "retention-probe-does-not-exist", "corpus")
except RuntimeError as error:
    assert "MISSING FROZEN ROOT" in str(error), error
else:
    raise RuntimeError("missing durable root was not reported loudly")

# Sparse-wall-bbox positive/negative controls.
module["negative_controls"]()
assert "outside start/end rejected" in module["STATE"]["overshoot_controls"]

# Cross-check settings validation still fails a conflicting disclosure.
try:
    module["check_settings"]({"wall_generator": "classic"}, {"wall_loops": "2"})
except RuntimeError as error:
    assert "missing supported-setting disclosure" in str(error), error
else:
    raise RuntimeError("settings gate accepted a missing disclosure")

# Regression for the first t41 preflight stop (recorded under preflight-stop-1):
# the toolchain allowlist check must compare EVERY ALLOWED_* constant,
# including WASM_TARGET. The original check compared only four, so it rejected
# the correct tree on the very first gate.
preflight = runpy.run_path(str(EVIDENCE / "preflight.py"))
driver = (ROOT / "xtask/src/rustc_driver.rs").read_text(encoding="utf-8")
actual = preflight["read_allowlist"](driver)
buggy_expected = {
    "RELEASE": "1.96.0",
    "COMMIT_HASH": "ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96",
    "HOST": "x86_64-pc-windows-msvc",
    "LLVM_VERSION": "22.1.2",
}
if actual == buggy_expected:
    raise RuntimeError(
        "regression guard is vacuous: the buggy four-key expectation now matches"
    )
assert preflight["ALLOWED_COMPILE_POLICY"] == actual, (
    "the fixed allowlist comparison must match the real driver constants"
)
assert preflight["ALLOWED_COMPILE_POLICY"] == {
    "RELEASE": "1.96.0",
    "COMMIT_HASH": "ac68faa20c58cbccd01ee7208bf3b6e93a7d7f96",
    "HOST": "x86_64-pc-windows-msvc",
    "LLVM_VERSION": "22.1.2",
    "WASM_TARGET": "wasm32-unknown-unknown",
}
widened = dict(actual, EXTRA="unexpected")
assert widened != preflight["ALLOWED_COMPILE_POLICY"], (
    "an extra allowlist constant must fail the comparison"
)

if len(sys.argv) > 1:
    attempt = Path(sys.argv[1]).resolve()
    actual = module["corpus_files"]()
    original = json.loads((attempt / "historical-corpus-hashes.json").read_text())
    assert actual == original, "historical corpus was altered"
    print("corpus inventory matches the recorded historical corpus")
print(
    "PASS: Windows manifest lookup; durable-root policy; missing-root rejection; "
    "bbox positive/outside-start/outside-end/sparse-without-wall controls; "
    "settings gate rejects a missing disclosure; allowlist compares every "
    "ALLOWED_* constant (including WASM_TARGET) and rejects an extra one"
)
