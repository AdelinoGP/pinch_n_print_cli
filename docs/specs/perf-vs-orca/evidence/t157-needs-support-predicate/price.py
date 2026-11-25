#!/usr/bin/env python3
"""Price the t157 needs_support fast-path predicate against the t154 ledger.

Reads:
  - docs/specs/perf-vs-orca/evidence/t157-needs-support-predicate/probe.json
  - docs/specs/perf-vs-orca/evidence/t154-benchy-per-layer/arithmetic.json
  - docs/specs/perf-vs-orca/evidence/t154-benchy-per-layer/carriers.json

Writes:
  - docs/specs/perf-vs-orca/evidence/t157-needs-support-predicate/summary.json
"""

from __future__ import annotations

import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
T154 = HERE.parent / "t154-benchy-per-layer"

CELL_ORDER = [
    "benchy-classic-off",
    "benchy-classic-on",
    "base-classic-off",
    "base-classic-on",
]


def main() -> int:
    probe = json.loads((HERE / "probe.json").read_text(encoding="utf-8"))
    t154_arith = json.loads((T154 / "arithmetic.json").read_text(encoding="utf-8"))
    t154_carriers = json.loads((T154 / "carriers.json").read_text(encoding="utf-8"))

    arith_by_cell = t154_arith["cells"]
    carriers_by_cell = t154_carriers["cells"]

    cells_summary = {}
    total_dual_calls = 0
    total_dual_mismatches = 0

    for cell in CELL_ORDER:
        dual = probe["cells"][cell]["dual"]
        fast = probe["cells"][cell]["fast"]
        ar = arith_by_cell[cell]
        ca = carriers_by_cell[cell]
        eff_p = float(ca["effective_parallelism"])

        ns_dual = dual["ns_dual_by_caller"]
        ns_fast = fast["ns_fast_by_caller"]

        # Aggregate per-caller dual stats
        per_layer_callers = ["Layer::Perimeters", "Layer::Infill"]
        per_layer_full_us = sum(
            ns_dual[c]["full_us_total"] for c in per_layer_callers if c in ns_dual
        )
        per_layer_fast_us_in_dual = sum(
            ns_dual[c]["fast_us_total"] for c in per_layer_callers if c in ns_dual
        )
        per_layer_fast_us_in_fast = sum(
            ns_fast[c]["fast_us_total"] for c in per_layer_callers if c in ns_fast
        )
        per_layer_saved_us_dual = per_layer_full_us - per_layer_fast_us_in_dual

        sa_dual = ns_dual.get("PrePass::SupportAnalysis")
        sa_fast = ns_fast.get("PrePass::SupportAnalysis")
        sa_full_us = sa_dual["full_us_total"] if sa_dual else 0
        sa_fast_us_in_dual = sa_dual["fast_us_total"] if sa_dual else 0
        sa_fast_us_in_fast = sa_fast["fast_us_total"] if sa_fast else 0
        sa_saved_us_dual = sa_full_us - sa_fast_us_in_dual

        all_full_us = per_layer_full_us + sa_full_us
        all_fast_us_dual = per_layer_fast_us_in_dual + sa_fast_us_in_dual
        all_saved_us_dual = all_full_us - all_fast_us_dual

        cell_calls = sum(v["calls"] for v in ns_dual.values())
        cell_mismatches = sum(v["mismatches"] for v in ns_dual.values())
        total_dual_calls += cell_calls
        total_dual_mismatches += cell_mismatches

        # Prep views comparison between dual and fast probe passes
        prep_perim_dual_ms = dual["per_layer_stages"]["Layer::Perimeters"][
            "prep_views_ms"
        ]
        prep_perim_fast_ms = fast["per_layer_stages"]["Layer::Perimeters"][
            "prep_views_ms"
        ]
        prep_infill_dual_ms = dual["per_layer_stages"]["Layer::Infill"]["prep_views_ms"]
        prep_infill_fast_ms = fast["per_layer_stages"]["Layer::Infill"]["prep_views_ms"]

        prep_total_dual_ms = round(prep_perim_dual_ms + prep_infill_dual_ms, 3)
        prep_total_fast_ms = round(prep_perim_fast_ms + prep_infill_fast_ms, 3)
        prep_saved_ms = round(prep_total_dual_ms - prep_total_fast_ms, 3)

        # Ledger pricing at t154's effective parallelism
        per_layer_saved_ms = round(per_layer_saved_us_dual / 1000.0, 3)
        per_layer_wall_eq_saved_ms = round(per_layer_saved_ms / eff_p, 1)
        per_layer_wall_eq_saved_s = round(per_layer_saved_ms / 1000.0 / eff_p, 4)

        # PrePass::SupportAnalysis eligibility wall saving (supports-on only)
        sa_elig_dual_wall_ms = (
            dual["sa_eligibility"]["wall_ms"] if dual["sa_eligibility"] else 0.0
        )
        sa_elig_fast_wall_ms = (
            fast["sa_eligibility"]["wall_ms"] if fast["sa_eligibility"] else 0.0
        )
        sa_elig_wall_saved_ms = round(
            max(0.0, sa_elig_dual_wall_ms - sa_elig_fast_wall_ms), 3
        )
        if sa_dual and (sa_full_us + sa_fast_us_in_dual) > 0:
            sa_eff_p = round(
                (sa_full_us + sa_fast_us_in_dual)
                / max(dual["sa_eligibility"]["wall_us"], 1),
                2,
            )
            sa_clean_full_wall_est_ms = round(sa_full_us / 1000.0 / sa_eff_p, 1)
            sa_clean_fast_wall_est_ms = round(sa_fast_us_in_fast / 1000.0 / sa_eff_p, 1)
            sa_clean_wall_saved_est_ms = round(
                sa_clean_full_wall_est_ms - sa_clean_fast_wall_est_ms, 1
            )
        else:
            sa_eff_p = None
            sa_clean_full_wall_est_ms = 0.0
            sa_clean_fast_wall_est_ms = 0.0
            sa_clean_wall_saved_est_ms = 0.0

        total_wall_eq_saved_ms = round(
            per_layer_wall_eq_saved_ms + sa_clean_wall_saved_est_ms, 1
        )
        total_wall_eq_saved_s = round(total_wall_eq_saved_ms / 1000.0, 4)

        pnp_s = float(ar["pnp_s"])
        orca_s = float(ar["orca_s"])
        projected_pnp_s = round(pnp_s - total_wall_eq_saved_s, 4)
        projected_ratio = round(projected_pnp_s / orca_s, 4)
        pct_of_pnp_wall = round(100.0 * total_wall_eq_saved_s / pnp_s, 2)

        # Observed probe phase delta (attribution-only sanity check)
        probe_per_layer_dual_ms = dual["phases_ms"]["per_layer"]
        probe_per_layer_fast_ms = fast["phases_ms"]["per_layer"]
        probe_per_layer_delta_ms = probe_per_layer_dual_ms - probe_per_layer_fast_ms

        cells_summary[cell] = {
            "layer_count": dual["slice_stats"]["layer_count"],
            "effective_parallelism_t154": eff_p,
            "baseline_pnp_s": pnp_s,
            "baseline_orca_s": orca_s,
            "baseline_ratio": round(float(ar["ratio_of_record"]), 4),
            "predicate_calls_total": cell_calls,
            "predicate_mismatches": cell_mismatches,
            "per_caller_dual": ns_dual,
            "per_caller_fast": ns_fast,
            "thread_totals_ms": {
                "per_layer_full_ms": round(per_layer_full_us / 1000.0, 3),
                "per_layer_fast_in_dual_ms": round(
                    per_layer_fast_us_in_dual / 1000.0, 3
                ),
                "per_layer_fast_in_fast_ms": round(
                    per_layer_fast_us_in_fast / 1000.0, 3
                ),
                "per_layer_saved_ms": per_layer_saved_ms,
                "support_analysis_full_ms": round(sa_full_us / 1000.0, 3),
                "support_analysis_fast_in_dual_ms": round(
                    sa_fast_us_in_dual / 1000.0, 3
                ),
                "support_analysis_fast_in_fast_ms": round(
                    sa_fast_us_in_fast / 1000.0, 3
                ),
                "support_analysis_saved_ms": round(sa_saved_us_dual / 1000.0, 3),
                "all_sites_full_ms": round(all_full_us / 1000.0, 3),
                "all_sites_fast_in_dual_ms": round(all_fast_us_dual / 1000.0, 3),
                "all_sites_saved_ms": round(all_saved_us_dual / 1000.0, 3),
                "all_sites_speedup_ratio": round(
                    all_full_us / max(all_fast_us_dual, 1), 2
                ),
            },
            "prep_views_thread_totals_ms": {
                "perimeters_dual_ms": prep_perim_dual_ms,
                "perimeters_fast_ms": prep_perim_fast_ms,
                "infill_dual_ms": prep_infill_dual_ms,
                "infill_fast_ms": prep_infill_fast_ms,
                "combined_dual_ms": prep_total_dual_ms,
                "combined_fast_ms": prep_total_fast_ms,
                "combined_saved_ms": prep_saved_ms,
                "residual_non_needs_support_prep_ms": prep_total_fast_ms,
            },
            "priced_wall_savings": {
                "per_layer_wall_eq_saved_ms": per_layer_wall_eq_saved_ms,
                "per_layer_wall_eq_saved_s": per_layer_wall_eq_saved_s,
                "support_analysis_eff_parallelism": sa_eff_p,
                "support_analysis_dual_elig_wall_ms": sa_elig_dual_wall_ms,
                "support_analysis_fast_elig_wall_ms": sa_elig_fast_wall_ms,
                "support_analysis_probe_elig_wall_delta_ms": sa_elig_wall_saved_ms,
                "support_analysis_clean_full_wall_est_ms": sa_clean_full_wall_est_ms,
                "support_analysis_clean_fast_wall_est_ms": sa_clean_fast_wall_est_ms,
                "support_analysis_wall_eq_saved_ms": sa_clean_wall_saved_est_ms,
                "total_wall_eq_saved_ms": total_wall_eq_saved_ms,
                "total_wall_eq_saved_s": total_wall_eq_saved_s,
                "pct_of_baseline_pnp_wall": pct_of_pnp_wall,
                "projected_pnp_s": projected_pnp_s,
                "projected_ratio": projected_ratio,
            },
            "probe_phase_sanity_check_attribution_only": {
                "per_layer_phase_dual_ms": probe_per_layer_dual_ms,
                "per_layer_phase_fast_ms": probe_per_layer_fast_ms,
                "per_layer_phase_delta_ms": probe_per_layer_delta_ms,
                "slice_complete_dual_ms": dual["slice_complete"]["elapsed_ms"],
                "slice_complete_fast_ms": fast["slice_complete"]["elapsed_ms"],
                "slice_complete_delta_ms": (
                    dual["slice_complete"]["elapsed_ms"]
                    - fast["slice_complete"]["elapsed_ms"]
                ),
            },
        }

    # Check stop rule for graduating t158:
    # >= 1.5 s wall-equivalent on base OR >= 150 ms wall-equivalent on benchy
    base_max_wall_saved_s = max(
        cells_summary["base-classic-off"]["priced_wall_savings"][
            "total_wall_eq_saved_s"
        ],
        cells_summary["base-classic-on"]["priced_wall_savings"][
            "total_wall_eq_saved_s"
        ],
    )
    benchy_max_wall_saved_ms = max(
        cells_summary["benchy-classic-off"]["priced_wall_savings"][
            "total_wall_eq_saved_ms"
        ],
        cells_summary["benchy-classic-on"]["priced_wall_savings"][
            "total_wall_eq_saved_ms"
        ],
    )
    passes_base_gate = base_max_wall_saved_s >= 1.5
    passes_benchy_gate = benchy_max_wall_saved_ms >= 150.0

    summary = {
        "ticket": 157,
        "slug": "needs-support-predicate-attribution",
        "kind": "attribution_only",
        "head_commit": "c74e64ec",
        "reader_census": {
            "production_decision_consumers": [
                {
                    "crate": "slicer-runtime",
                    "file": "crates/slicer-runtime/src/builtins/support_analysis_producer.rs",
                    "symbol": "commit_support_analysis_builtin",
                    "gating": "enable_support == true && support_type.is_auto()",
                    "call_kind": "Direct host call: SliceRegionView::from_ir(region, 0.0, Vec::new()).derive_needs_support(surface_classification.as_deref())",
                }
            ],
            "per_layer_view_preparation_evaluating_derive_needs_support": [
                {
                    "crate": "slicer-wasm-host",
                    "file": "crates/slicer-wasm-host/src/marshal/prepared.rs",
                    "symbol": "prepare_regions",
                    "called_from": "crates/slicer-runtime/src/layer_executor.rs::prepare_views_for_layer",
                    "stages_triggering_evaluation": [
                        "Layer::Perimeters (via prepare_perimeter_source_region_views)",
                        "Layer::Infill (via prepare_ordinary_region_views after commit_perimeter invalidates cached views)",
                    ],
                }
            ],
            "guest_module_readers_of_slice_region_view_needs_support": [],
            "retired_guest_readers": [
                {
                    "crate": "tree-support",
                    "file": "modules/core-modules/tree-support/src/lib.rs",
                    "symbol": "TreeSupport::run_support",
                    "note": "Explicitly retired region.needs_support() from SupportPaintPolicy::DefaultEligible in Packet 224 because PrePass::SupportAnalysis already gates auto-support contact detection on needs_support.",
                }
            ],
            "test_only_readers_of_needs_support": [
                "crates/slicer-runtime/tests/contract/region_preparation_sharing_tdd.rs",
                "crates/slicer-runtime/tests/executor/region_eligibility_tdd.rs",
                "crates/slicer-runtime/tests/executor/view_seam_identity_tdd.rs",
                "crates/slicer-runtime/tests/integration/source_identity_repair_tdd.rs",
                "crates/slicer-runtime/tests/unit/blackboard_layer_arena_tdd.rs",
                "crates/slicer-sdk/tests/layer_module_tdd.rs",
            ],
        },
        "equivalence_verification": {
            "total_dual_calls": total_dual_calls,
            "total_dual_mismatches": total_dual_mismatches,
            "gcode_reference_set": "docs/specs/perf-vs-orca/evidence/t151-skirt-hull-contour/reference.json",
            "all_8_runs_matched_reference": True,
        },
        "stop_rule_evaluation": {
            "base_threshold_s": 1.5,
            "base_max_priced_wall_saved_s": base_max_wall_saved_s,
            "passes_base_gate": passes_base_gate,
            "benchy_threshold_ms": 150.0,
            "benchy_max_priced_wall_saved_ms": benchy_max_wall_saved_ms,
            "passes_benchy_gate": passes_benchy_gate,
            "graduate_phase2_ticket_158": passes_base_gate or passes_benchy_gate,
            "recommended_phase2_shape": (
                "Option A: Exact fast-path boolean intersection predicate inside "
                "SliceRegionView::derive_needs_support (strict bbox overlap prefilter "
                "+ strict-interior point-in-ExPolygon witness in i128, with Clipper2 "
                "intersection_ex fallback on overlapping pairs only). Accelerates all "
                "three call sites (PrePass::SupportAnalysis, Layer::Perimeters, "
                "Layer::Infill) by 91x-134x with zero WIT/schema/lifecycle churn."
            ),
        },
        "cells": cells_summary,
    }

    (HERE / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print("Wrote summary.json")
    for cell in CELL_ORDER:
        cs = cells_summary[cell]
        pw = cs["priced_wall_savings"]
        tt = cs["thread_totals_ms"]
        print(
            f"  {cell}: full={tt['all_sites_full_ms']:.1f}ms -> "
            f"fast={tt['all_sites_fast_in_dual_ms']:.1f}ms "
            f"({tt['all_sites_speedup_ratio']}x), "
            f"priced_wall_saved={pw['total_wall_eq_saved_ms']:.1f}ms "
            f"({pw['pct_of_baseline_pnp_wall']:.2f}% of {cs['baseline_pnp_s']}s), "
            f"proj_pnp={pw['projected_pnp_s']}s ({pw['projected_ratio']}x)"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
