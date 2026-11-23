"""One-shot paired-mode candidate build, with actual dependency-graph audits."""

import json
import shutil
import subprocess
import sys
import manage as m

identity = json.loads((m.HERE / "identity.json").read_text())
assert (
    m.sha(m.LIB / "src/engine_fns.rs")
    == identity["registry_inventory"]["src/engine_fns.rs"]
), "original top_x changed"
resume = sys.argv[1:] == ["--resume-after-link-path-diagnosis"]
assert not sys.argv[1:] or resume, "unknown resume route"
report = (
    json.loads((m.HERE / "candidate-builds.json").read_text())
    if resume
    else {"snapshots": {}, "freshness": {}, "diagnose": {}}
)
for mode, flags in m.MODES:
    snapshot = m.RAW / "snapshots" / (mode + "-candidate")
    if resume and mode == "ordinary":
        assert m.inv(snapshot) == report["snapshots"][mode], "completed snapshot drift"
        continue
    assert not snapshot.exists(), "one-shot candidate snapshot"
    m.run(
        ["cargo", "xtask", "dist", "--edition", "developer", *flags],
        mode
        + ("-candidate-build-link-path-resolved" if resume else "-candidate-build"),
        m.SOURCE,
    )
    report["freshness"][mode] = m.run(
        ["cargo", "xtask", "build-guests", *flags, "--check"],
        mode + "-candidate-freshness",
        m.SOURCE,
    )
    dist = m.SOURCE / (
        "target/dist/developer"
        if mode == "ordinary"
        else "target/dist-accelerated/developer"
    )
    shutil.copytree(dist, snapshot)
    report["snapshots"][mode] = m.inv(snapshot)
    m.run(
        [
            snapshot / "pnp_cli.exe",
            "module",
            "diagnose",
            "--module-dir",
            snapshot / "modules",
            "--no-default-module-paths",
            "--no-integrated-modules",
        ],
        mode + "-candidate-diagnose",
    )
    diagnosis = json.loads(
        (m.HERE / "logs" / (mode + "-candidate-diagnose.log")).read_text()
    )
    assert (
        diagnosis["pass"]
        and diagnosis["modules"]
        and all(p["provenance"] == "external" for p in diagnosis["modules"])
    )
    report["diagnose"][mode] = diagnosis
    m.write(m.HERE / "candidate-builds.json", report)
subprocess.run([sys.executable, str(m.HERE / "audit-resolutions.py")], check=True)
print(
    "PASS: paired-mode external candidate snapshots and exact-version graph resolutions"
)
