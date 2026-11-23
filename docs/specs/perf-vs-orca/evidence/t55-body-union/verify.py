"""Output qualification uses the audited streaming scanner from t53."""

import importlib.util
import json
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location(
    "t53_verify", HERE.parent / "t53-union-simplify/verify.py"
)
base = importlib.util.module_from_spec(spec)
spec.loader.exec_module(base)
output_row = base.output_row


def native():
    import corpus

    evidence = json.loads((HERE / "evidence.json").read_text())
    selected = {
        (job, c["context"]["layer"]): c
        for job in ("benchy", "base")
        for c in evidence["ordinary-" + job]["cases"]
    }
    for report_name, log_name in (
        ("native-internals-first.json", "native-replay.log"),
        ("native-internals.json", "native-counters-replay.log"),
    ):
        report = json.loads((HERE / report_name).read_text())
        with (HERE / log_name).open() as stream:
            logged = [
                json.loads(line.partition(" ")[2])
                for line in stream
                if line.startswith("PERF-T55-REPLAY ")
            ]
        assert logged == [report], "native report/log drift"
        assert {(c["job"], c["layer"]) for c in report["cases"]} == set(selected)
        assert len(report["cases"]) == len(selected)
        for c in report["cases"]:
            s = selected[(c["job"], c["layer"])]
            assert (
                c["input"] == s["body_carved"][:3] and c["union"] == s["body_union"][:3]
            )
            assert c["full_roles"] == s["returned_roles"]
            assert (
                c["flatten_ns"] + c["boolean_wrapper_ns"] + c["reconstruct_ns"]
                <= c["total_ns"]
            )
            assert (
                sum(
                    c[k]
                    for k in (
                        "add_paths_ns",
                        "execute_internal_ns",
                        "build_tree_ns",
                        "cleanup_ns",
                    )
                )
                <= c["boolean_wrapper_ns"]
            )
            assert (
                sum(
                    c[k]
                    for k in (
                        "sweep_reset_ns",
                        "sweep_insert_ns",
                        "sweep_horizontal_ns",
                        "sweep_intersections_ns",
                        "sweep_top_ns",
                        "sweep_final_horz_joins_ns",
                    )
                )
                <= c["execute_internal_ns"]
            )
            assert (
                c["intersection_build_ns"] + c["intersection_process_ns"]
                <= c["sweep_intersections_ns"]
            )
            if "adjust_copy_ns" in c:
                assert c["adjust_copy_ns"] <= c["intersection_build_ns"]
                assert (
                    0
                    < c["unchanged_edge_repeated_top_x_calls"]
                    <= c["top_nonterminal_top_x_calls"]
                    <= c["adjust_top_x_calls"]
                )
                assert (
                    c["repeated_top_x_mismatches"] == 0 and c["merge_comparisons"] > 0
                )
        holes = sum(s["body_union"][1] > 0 for s in selected.values())
        cuts = sum(c["changed"] for s in selected.values() for c in s["cuts"])
        assert report["hole_cases"] == holes and report["positive_body_cuts"] == cuts
        assert report["negative_controls"] == {
            "vertex": len(selected),
            "hole": holes,
            "role": len(selected),
        }
    provenance = json.loads((HERE / "harness-provenance.json").read_text())
    assert (
        provenance["no_new_dependency_versions"]
        and provenance["internals_patch_reapplied_exactly"]
    )
    assert provenance["resolved_clipper"][0]["version"] == "1.1.0"
    assert corpus.sha(HERE / "harness.rs") == provenance["harness_sha256"]
    assert corpus.sha(HERE / "harness.Cargo.lock") == provenance["lock_sha256"]
    # Regenerate the streamed numeric framing, including all original f32 bit patterns.
    with tempfile.TemporaryDirectory(dir=HERE, prefix="verify-framing-") as tmp:
        corpus.replay_text(Path(tmp))
        for job in ("benchy", "base"):
            assert corpus.sha(Path(tmp) / (job + "-replay.txt")) == corpus.sha(
                HERE / (job + "-replay.txt")
            )
    validation = json.loads((HERE / "validation.json").read_text())
    for path, digest in validation["sealed_evidence"].items():
        assert corpus.sha(HERE / path) == digest, "sealed evidence drift: " + path
    assert all(c["equal"] for c in validation["streamed_output_comparisons"].values())
    assert all(c["exit_code"] == 0 for c in validation["final_freshness"].values())
    print(
        "PASS: native log/provenance, nested phase accounting, controls and exact replay framing"
    )


if __name__ == "__main__":
    import corpus

    corpus.verify()
    native()
