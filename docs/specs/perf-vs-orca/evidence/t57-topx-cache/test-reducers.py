"""Synthetic regression inputs only: not performance or qualification evidence."""

import copy
import tempfile
from pathlib import Path
import unittest
import campaign
import outputs


def synthetic_rows():
    proof = {
        "sha256": "a" * 64,
        "bytes": 1,
        "type_counts": {"Outer wall": 1},
        "warnings": {},
    }
    rows = []
    for pair in range(7):
        order = (
            ["candidate", "baseline"]
            if pair and not pair % 2
            else ["baseline", "candidate"]
        )
        for arm in order:
            wall, cpu = (10.0, 20.0) if arm == "baseline" else (9.0, 18.0)
            rows.append(
                {
                    "arm": arm,
                    "pair": pair,
                    "warmup": int(pair == 0),
                    "sha256": proof["sha256"],
                    "bytes": 1,
                    "type_counts": proof["type_counts"],
                    "output": {"warnings": {}},
                    "threads": 12,
                    "status": "ok",
                    "degraded": False,
                    "exit_code": 0,
                    "fatal": 0,
                    "non_fatal": 0,
                    "wall_seconds": wall,
                    "cpu_seconds": cpu,
                    "cpu_wall_ratio": cpu / wall,
                }
            )
    return rows, proof


class ReducerTests(unittest.TestCase):
    def test_independent_analytic_favorable_gate(self):
        rows, proof = synthetic_rows()
        summary = campaign.summarize(rows, proof)
        self.assertTrue(summary["gate_passed"])
        self.assertEqual(summary["retained_pairs"], 6)
        self.assertEqual(
            summary["paired_median_deltas"], {"wall_seconds": -1.0, "cpu_seconds": -2.0}
        )

    def test_whole_pair_starvation_and_no_resampling(self):
        rows, proof = synthetic_rows()
        for row in rows:
            if row["arm"] == "candidate" and row["pair"] in (1, 2):
                row["wall_seconds"] = 30.0
                row["cpu_wall_ratio"] = row["cpu_seconds"] / 30.0
        summary = campaign.summarize(rows, proof)
        self.assertFalse(summary["gate_passed"])
        self.assertEqual(summary["status"], "inconclusive")
        self.assertEqual(summary["retained_pairs"], 4)
        self.assertEqual([p["pair"] for p in summary["excluded_pairs"]], [1, 2])

    def test_wall_win_cannot_hide_cpu_regression(self):
        rows, proof = synthetic_rows()
        for row in rows:
            if row["arm"] == "candidate":
                row["cpu_seconds"] = 22.0
                row["cpu_wall_ratio"] = 22.0 / row["wall_seconds"]
        self.assertFalse(campaign.summarize(rows, proof)["gate_passed"])

    def test_output_and_order_sabotage(self):
        rows, proof = synthetic_rows()
        bad = copy.deepcopy(rows)
        bad[2]["sha256"] = "0" * 64
        with self.assertRaisesRegex(AssertionError, "output drift"):
            campaign.summarize(bad, proof)
        bad = copy.deepcopy(rows)
        bad[2], bad[3] = bad[3], bad[2]
        with self.assertRaisesRegex(AssertionError, "paired order"):
            campaign.summarize(bad, proof)

    def test_outcome_taxonomy_never_conflates_inconclusive_with_drop(self):
        self.assertEqual(campaign.decide({c: "pass" for c in campaign.CELLS}), "KEEP")
        stopped = dict.fromkeys(campaign.CELLS, "not_run")
        stopped[campaign.CELLS[0]] = "pass"
        stopped[campaign.CELLS[1]] = "fail"
        self.assertEqual(campaign.decide(stopped), "DROP")
        stopped[campaign.CELLS[1]] = "inconclusive"
        self.assertEqual(campaign.decide(stopped), "INCONCLUSIVE")
        with self.assertRaisesRegex(AssertionError, "not_run without a recorded stop"):
            campaign.decide(dict.fromkeys(campaign.CELLS, "not_run"))
        illegal = dict.fromkeys(campaign.CELLS, "pass")
        illegal[campaign.CELLS[0]] = "fail"
        with self.assertRaisesRegex(AssertionError, "run after stop"):
            campaign.decide(illegal)
        illegal[campaign.CELLS[2]] = "fail"
        illegal[campaign.CELLS[0]] = "pass"
        with self.assertRaisesRegex(AssertionError, "run after stop"):
            campaign.decide(illegal)

    def test_real_streamed_comparison_rejects_vertex_byte_drift(self):
        with tempfile.TemporaryDirectory(
            dir=r"C:\Users\agpen\AppData\Local\Temp\opencode"
        ) as directory:
            a, b = Path(directory) / "a", Path(directory) / "b"
            a.write_bytes(b"G1 X1 Y2 E1\n")
            b.write_bytes(b"G1 X1 Y3 E1\n")
            with self.assertRaisesRegex(AssertionError, "bytes differ"):
                outputs.equal_files(a, b)

    def test_cross_cell_relabel_and_identity_mismatch_are_rejected(self):
        rows, proof = synthetic_rows()
        cell = "ordinary-benchy-classic-on"
        for row in rows:
            row.update(
                mode="ordinary",
                fixture="benchy",
                generator="classic",
                supports="on",
                phase="measure",
                run_id=cell + "-" + row["arm"] + "-" + str(row["pair"]),
            )
        self.assertTrue(campaign.summarize(rows, proof, cell)["gate_passed"])
        relabeled = copy.deepcopy(rows)
        for row in relabeled:
            row["mode"] = "accelerated"
        with self.assertRaisesRegex(AssertionError, "identity mismatch"):
            campaign.summarize(relabeled, proof, cell)
        mismatched = copy.deepcopy(rows)
        mismatched[2]["run_id"] = "accelerated-benchy-classic-on-baseline-1"
        with self.assertRaisesRegex(AssertionError, "run-id mismatch"):
            campaign.summarize(mismatched, proof, cell)

    def test_disclosure_rejects_status_generator_and_vacuous_support(self):
        # Synthetic M83 fixture and independently specified completion contract.
        import json

        with tempfile.TemporaryDirectory(
            dir=r"C:\Users\agpen\AppData\Local\Temp\opencode"
        ) as directory:
            out, log = Path(directory) / "output", Path(directory) / "stderr"
            gcode = "; wall_generator = classic\nM83\n" + "".join(
                ";TYPE:" + role + "\nG1 X1 Y2 E1\n"
                for role in (
                    "Outer wall",
                    "Sparse infill",
                    "Support",
                    "Support interface",
                )
            )
            out.write_text(gcode)
            event = {
                "event": "slice_complete",
                "status": "ok",
                "degraded": False,
                "fatal_error_count": 0,
                "non_fatal_error_count": 0,
            }
            holder = "WARN claim 'perimeter-generator' already held by 'com.core.classic-perimeters'\n"
            log.write_text(holder + json.dumps(event) + "\n")
            disclosed = outputs.disclosure(out, log, "classic", "on")
            self.assertEqual(disclosed["extruding_xy_moves"]["Support interface"], 1)
            with self.assertRaisesRegex(AssertionError, "generator marker"):
                outputs.disclosure(out, log, "arachne", "on")
            event["non_fatal_error_count"] = 1
            log.write_text(holder + json.dumps(event) + "\n")
            with self.assertRaisesRegex(AssertionError, "unclean completion"):
                outputs.disclosure(out, log, "classic", "on")
            event["non_fatal_error_count"] = 0
            log.write_text(holder + json.dumps(event) + "\n")
            out.write_text(
                gcode.replace(
                    ";TYPE:Support interface\nG1 X1 Y2 E1\n",
                    ";TYPE:Support interface\n",
                )
            )
            with self.assertRaisesRegex(AssertionError, "nonvacuous support"):
                outputs.disclosure(out, log, "classic", "on")


if __name__ == "__main__":
    unittest.main(verbosity=2)
