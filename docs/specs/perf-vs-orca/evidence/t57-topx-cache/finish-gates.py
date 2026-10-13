"""Code gates and reconstructible patch/provenance, prior to any slice campaign."""

import difflib
import json
from pathlib import Path
import shutil
import manage as m

identity = json.loads((m.HERE / "identity.json").read_text())
assert all(m.sha(m.REPO / p) == h for p, h in identity["build_inputs"].items())
assert m.inv(Path(identity["registry_path"])) == identity["registry_inventory"]
gates = {}
for label, command in [
    ("candidate-workspace-build", ["cargo", "build", "--workspace"]),
    (
        "candidate-workspace-all-targets-clippy",
        ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ),
    ("candidate-literals", ["cargo", "xtask", "check-literals"]),
    ("candidate-test-quality", ["cargo", "xtask", "check-test-quality", "--report"]),
    (
        "ordinary-final-candidate-freshness",
        ["cargo", "xtask", "build-guests", "--check"],
    ),
    (
        "accelerated-final-candidate-freshness",
        ["cargo", "xtask", "build-guests", "--accelerated", "--check"],
    ),
]:
    gates[label] = {"command": command, "exit_code": m.run(command, label, m.SOURCE)}
    m.write(m.HERE / "code-gates.json", gates)
m.run(["python", m.HERE / "test-reducers.py"], "reducer-regressions")
m.run(
    [
        "pwsh",
        "-NoProfile",
        "-Command",
        "$tokens=$null; $errors=$null; [System.Management.Automation.Language.Parser]::ParseFile('"
        + str(m.HERE / "run-cell.ps1")
        + "',[ref]$tokens,[ref]$errors)|Out-Null; if($errors.Count){$errors|Format-List; exit 1}; 'PASS: runner syntax'",
    ],
    "runner-syntax",
)
diffs, originals, candidate_files = [], {}, {}
for relative, digest in identity["source_snapshot"].items():
    old, new = m.REPO / relative, m.SOURCE / relative
    assert m.sha(old) == digest, "production source snapshot drift"
    if m.sha(new) == digest:
        continue
    originals[relative], candidate_files[relative] = digest, m.sha(new)
    diffs.extend(
        difflib.unified_diff(
            old.read_text().splitlines(True),
            new.read_text().splitlines(True),
            fromfile="a/" + relative,
            tofile="b/" + relative,
        )
    )
registry = Path(identity["registry_path"])
for relative in sorted(set(identity["registry_inventory"]) | set(m.inv(m.LIB))):
    old, new = registry / relative, m.LIB / relative
    if old.is_file() and new.is_file() and m.sha(old) == m.sha(new):
        continue
    path = ".local-libs/clipper2-rust/" + relative
    originals[path] = m.sha(old) if old.is_file() else None
    candidate_files[path] = m.sha(new) if new.is_file() else None
    diffs.extend(
        difflib.unified_diff(
            old.read_text().splitlines(True) if old.is_file() else [],
            new.read_text().splitlines(True) if new.is_file() else [],
            fromfile="a/" + path if old.is_file() else "/dev/null",
            tofile="b/" + path,
        )
    )
(m.HERE / "candidate.patch").write_text("".join(diffs))
m.write(
    m.HERE / "patch-provenance.json",
    {
        "original": originals,
        "candidate": candidate_files,
        "patch_sha256": m.sha(m.HERE / "candidate.patch"),
        "unchanged_top_x_sha256": m.sha(m.LIB / "src/engine_fns.rs"),
        "registry_unchanged": True,
        "production_build_inputs_unchanged": True,
    },
)
required = {
    "native-engine-checks.log": "1 passed; 0 failed",
    "wasm-engine-checks.log": "1056 independent",
    "native-role-replay.log": "1 passed; 0 failed",
    "native-consumers.log": "1 passed; 0 failed",
    "wasm-consumers-execution.log": "18 independent",
    "closure-witness.log": "1 passed; 0 failed",
    "reducer-regressions.log": "OK",
    "runner-syntax.log": "PASS",
}
for name, marker in required.items():
    assert marker in (m.HERE / "logs" / name).read_text(), (
        "missing actual exactness signal: " + name
    )
controls = json.loads((m.HERE / "negative-controls.json").read_text())
assert set(controls) == {"stale_y", "wrong_projection"}
assert all(
    v["native_assertion_rejected"] and v["wasm_assertion_rejected"]
    for v in controls.values()
)
accounting = json.loads((m.HERE / "accounting.json").read_text())
assert accounting["native"][3:] == accounting["wasm32"][3:] == [4, 1, 13]
m.write(
    m.HERE / "exactness.json",
    {
        "pass": True,
        "logs": {name: m.sha(m.HERE / "logs" / name) for name in required},
        "native_wasm_engine_ordered_comparisons_each": 1056,
        "native_wasm_production_consumer_comparisons_each": 18,
        "recorded_full_role_cases": 8,
        "negative_controls": controls,
        "code_gates": gates,
        "candidate_patch_sha256": m.sha(m.HERE / "candidate.patch"),
        "coverage_limits": "selected recorded full-role cases only, not all-layer replay; no positive role simplification or populated BaseInterface in recorded cases; synthetic nested-hole meshes, not every mesh topology; layout accounting not peak RSS",
    },
)
print(
    "PASS: exactness, actual negative controls, code gates, freshness and reproducible candidate patch"
)
