"""Compare each actual root/independent guest graph with its OWN baseline authority."""

import json
import subprocess
import tomllib
from pathlib import Path
import manage as m


def lock_inventory(path):
    return sorted(
        [
            (p["name"], p["version"], p.get("source"), p.get("checksum"))
            for p in tomllib.loads(path.read_text())["package"]
            if p["name"] != "clipper2-rust"
        ],
        key=str,
    )


def normalized_graph(metadata):
    packages = {p["id"]: p for p in metadata["packages"]}

    def key(id):
        p = packages[id]
        return (
            p["name"],
            p["version"],
            None if p["name"] == "clipper2-rust" else p["source"],
        )

    result = {}
    for node in metadata["resolve"]["nodes"]:
        identity = json.dumps(key(node["id"]))
        assert identity not in result, "normalized package collision"
        result[identity] = {
            "features": sorted(node["features"]),
            "deps": sorted(
                [
                    {
                        "name": d["name"],
                        "package": key(d["pkg"]),
                        "kinds": d["dep_kinds"],
                    }
                    for d in node["deps"]
                ],
                key=lambda d: json.dumps(d, sort_keys=True),
            ),
        }
    return result


def audit():
    report = json.loads((m.HERE / "candidate-builds.json").read_text())
    assert set(report["snapshots"]) == {"ordinary", "accelerated"}
    resolutions, all_graphs = {}, {}
    manifests = [
        m.SOURCE / "Cargo.toml",
        *m.SOURCE.glob("modules/core-modules/*/wit-guest/Cargo.toml"),
        *m.SOURCE.glob("crates/slicer-wasm-host/test-guests/*/Cargo.toml"),
    ]
    for manifest in manifests:
        relative = manifest.relative_to(m.SOURCE)
        original = m.REPO / relative
        assert lock_inventory(manifest.with_name("Cargo.lock")) == lock_inventory(
            original.with_name("Cargo.lock")
        ), "actual per-graph lock drift"
        platforms = (
            ["x86_64-pc-windows-msvc", "wasm32-unknown-unknown"]
            if relative.as_posix() == "Cargo.toml"
            else ["wasm32-unknown-unknown"]
        )
        for platform in platforms:
            graphs = []
            for path, cwd in [(original, m.REPO), (manifest, m.SOURCE)]:
                graphs.append(
                    json.loads(
                        subprocess.check_output(
                            [
                                "cargo",
                                "metadata",
                                "--manifest-path",
                                str(path),
                                "--format-version",
                                "1",
                                "--offline",
                                "--locked",
                                "--filter-platform",
                                platform,
                            ],
                            cwd=cwd,
                        )
                    )
                )
            a, b = graphs
            assert normalized_graph(a) == normalized_graph(b), (
                "resolved package/edge/feature drift"
            )
            identifier = relative.as_posix() + ":" + platform
            all_graphs[identifier] = {
                "identical_nonengine_lock_inventory": True,
                "identical_normalized_resolved_packages_edges_features": True,
                "baseline_lock_sha256": m.sha(original.with_name("Cargo.lock")),
                "candidate_lock_sha256": m.sha(manifest.with_name("Cargo.lock")),
            }
            packages = [p for p in b["packages"] if p["name"] == "clipper2-rust"]
            if not packages:
                all_graphs[identifier]["engine_consumer"] = False
                continue
            assert len(packages) == 1 and packages[0]["version"] == "1.1.0"
            assert (
                m.LIB.joinpath("Cargo.toml").resolve()
                == Path(packages[0]["manifest_path"]).resolve()
            )
            node = next(
                n for n in b["resolve"]["nodes"] if n["id"] == packages[0]["id"]
            )
            assert "topx-test-support" not in node["features"], (
                "diagnostics leaked into production"
            )
            resolutions[identifier] = {
                "package": packages[0],
                "features": node["features"],
            }
            all_graphs[identifier]["engine_consumer"] = True
        m.write(m.HERE / "resolution-audit-progress.json", all_graphs)
    report["resolved_graphs"] = resolutions
    report["all_graphs"] = all_graphs
    report["candidate_library_inventory"] = m.inv(m.LIB)
    report["audit_correction"] = (
        "rejected root-union comparison; each independent graph's own unchanged baseline lock/resolution is the no-drift authority"
    )
    m.write(m.HERE / "candidate-builds.json", report)
    print(
        "PASS: actual per-graph package/edge/feature equality and exact private-engine integration; no diagnostic features"
    )


if __name__ == "__main__":
    audit()
