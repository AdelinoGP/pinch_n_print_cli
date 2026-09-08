# Design: 236-support-stabilization

## Controlling Code Paths

- Primary code path: `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` (`commit_support_analysis_builtin`, per-candidate `family_assignments` mint loop at lines ~253–279; two-stage lookup via `config_for_region_smallest_chain` → `support_family` with `"traditional"` fallback — the F-44 fix).
- Neighboring tests/fixtures: `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` (`planner_emits_one_entry_per_region_in_region_map` at line 503, registered bare in `tests/executor/main.rs:104`; fixture helpers `multi_region_map` / `multi_region_layer_plan` / `overhang_mesh` / local `run_prepass` at ~368); scheduler validation tests under `crates/slicer-scheduler`; tripwire in `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs:400`.
- OrcaSlicer comparison: see `requirements.md` §OrcaSlicer Reference Obligations; do not repeat delegation rules.

## Architecture Constraints

<!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and inspect its exit code: exit 0 means fresh, non-zero means stale (a distinct exit code signals `wasm-tools` is unavailable). Never use `rg -q 'STALE:'` — a `wasm-tools`-missing infrastructure error prints no `STALE:` and would read as fresh. If stale, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
<!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- The AC-8 fix changes which entries exist in `SupportAnalysisIR.family_assignments` (host IR, no WIT/schema bump): the SDK view type `SupportAnalysisView.family_assignments: Vec<SupportFamilyAssignment>` (`crates/slicer-sdk/src/prepass_types.rs:445–468`) already carries per-region rows keyed by `(object_id, region_id)` strings, so NO WIT edit is needed — but the guest-side minting parity (the tree planner's own `family_assignments` consumption) must be re-checked after the host change with fresh guests (first constraint bullet).
- Config keys are snake_case everywhere (E9). New `[config.schema]` entries + regenerated `docs/15_config_keys_reference.md` land in one commit (T8).
- Invariant 15 (plan §6): every RegionMap region gets exactly one attributed plan entry; regions requiring no support carry a structured no-work/declined record — never silence. Invariant 16: no acceptance command may match zero tests; every filter asserts a non-zero matched count.

## Plan Corrections

Two plan claims were falsified against the live tree during grounding (2026-08-22); downstream agents must use these corrected facts:

1. **Golden file location.** The plan §12 says `modules/core-modules/tree-support-planner/resources/golden/benchy_tree_support_regression_*`. On disk there is NO such directory; the tripwire resolves goldens from `<repo-root>/resources/golden/` (CARGO_MANIFEST_DIR up-walk in the test, lines ~454–462), where both files exist today. All rebless paths in this packet target workspace-root `resources/golden/benchy_tree_support_regression_branch_count.txt` / `_endpoints.txt`.
2. **Wedge regen gate is dead.** The plan §7 E3 names `SUPPORT_WEDGE_REGEN_GOLDEN=1` as the runtime-wedge gate; it has ZERO live-code hits (grep over crates/, modules/, docs only shows historical packets 119/121/122/210a/210b). The wedge golden pair was retired (TASK-163b-orca-ref note in `docs/07_implementation_status.md`). This packet's only live regeneration gate is `SUPPORT_PLANNER_REGEN_GOLDEN` (`orca_parity_tdd.rs:466`).

## Code Change Surface

- Selected approach: nine independent small fixes sharing one branch-green goal (per plan §12 brief), each behind its own step with its own narrow verification. No schema bumps, no WIT edits, no new public API except test-visible helpers.
- Exact functions, traits, manifests, tests, and fixtures:
  - `commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) — replace the per-candidate mint loop with a per-region walk over `blackboard.region_map()` regions for each `(object, layer)`; keep the existing two-stage family resolution; declined/no-candidate regions still receive exactly one structured assignment (Ruling 1 / invariant 15). The count assertion of `planner_emits_one_entry_per_region_in_region_map` stands unmodified.
  - `validate_startup_dag` helpers `validate_claim_conflicts` / `validate_write_conflicts` (`crates/slicer-scheduler/src/validation.rs`) — two changes. CLAIMS: add a family-scoped exemption set analogous to the fill-role exemption at line ~553 — claims `support-generator`, `support-planner`, `support-family:traditional`, `support-family:tree` are exempt in the GLOBAL pass only; the per-region pass stays untouched. WRITES: recognize the ADR-0059 write topology rather than exempting it — ADR-0059's decision clause states "the host aggregator is the sole writer of the aggregated `SupportPlanIR`, and family planners emit only family-scoped entries for host aggregation", so a planner-pair `WriteConflict` on `SupportPlanIR` / `SupportIR` where both modules hold family-scoped support claims is ORDERABLE-BY-AGGREGATION (the host aggregation edge is the ordering); implement this as an orderability recognition in the existing `left_transforms_right || right_transforms_left` check, NOT as a pair exemption. This preserves ADR-0059's host-aggregator sole-writer model verbatim — no ADR amendment and no deviation row are required (the plan's G-21 wording "update the validator contract to recognize family-scoped multi-holder claims" is satisfied at claim level; the IR-write noise disappears because the pairs become orderable). Genuine conflicts must still fire (`genuine_claim_conflict_still_rejected_after_family_exemption` authored red-first in Step 2; `genuine_write_conflict_still_rejected_after_aggregation_recognition` guards the write side).
  - `benchy_tree_support_regression_tripwire` (`modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs`) — build the fixture's `SupportGeometryView` from real occupancy polygons derived from `overhang_plate_fixture` instead of `entries: vec![]` (line ~421); classify drift vs old golden in writing; rebless once via `SUPPORT_PLANNER_REGEN_GOLDEN=1`. No planner source edits expected; if any are forced, STOP and report scope drift to the orchestrator.
  - `traditional-support-planner.toml` `[config.schema]` — add `support_threshold_angle` + alias `support_overhang_angle` (float, min 0, max 90, default 30.0, group "Support"). Host-side rejection path: config bounds check produces `ConfigResolutionError::OutOfRange` (mechanism exists: TASK-182 `ConfigBoundsIndex`, strictest-wins). Then regenerate docs via xtask.
  - `integrated_parity_harness.rs` (`crates/slicer-runtime/tests/common/`) — add `assert_guest_freshness(spec.wasm_path)` invoked inside `run_integrated_parity` before building bindings; reuse the freshness logic mirrored from `is_stale`'s newest-source-vs-artifact comparison (xtask `build_guests.rs`; the locator crate `pnp_cli_locator::staleness_reason` documents the mirror pattern). Four contract tests gain the assert transitively.
  - `native_and_wasm_layer_views_are_field_identical` (new, `crates/slicer-wasm-host`) — construct one representative layer view through `build_native_layer_request` (`marshal/native.rs:113`) and through the wasm dispatch projection; compare field-by-field. Shared-construction refactor only if it ends up smaller than the test.
  - `execute_paint_segmentation` BASE fallback (`crates/slicer-core/src/algos/paint_segmentation/mod.rs:857`) — derive fallback polygons from the object's own contours (mirror the multi-object branch's `per_object_contours.get(&rk.object_id)` scoping) so single-object layers stop using whole-layer all-object contours; new test pins it.
  - Docs/deletions: remove four draft dirs; update `docs/specs/support-generation-remediation-plan.md` rows 3–6 (lines 61–64); ADR-0059 status + `## Amendments`; gap-register re-measurement row; deviation-log rebless row (re-derive next DEV id at write time); `docs/04_host_scheduler.md` "Validation Passes" exemption note; `docs/07_implementation_status.md` registration via dispatch.
- Rejected alternatives and reasons:
  - Mesh-path-gate hypothesis for AC-8 (commit `c3c1ed5a`) — DISPROVED (plan T11); forbidden.
  - Host-side clamp instead of manifest declaration for G-22 — rejected: the register prefers either, but the manifest declaration keeps bounds data co-located with the module that reads the key and matches the 238a key-declaration pattern this queue establishes.
  - Reusing `SUPPORT_WEDGE_REGEN_GOLDEN` for the rebless — dead gate (see Plan Corrections).

## Files in Scope (read + edit)

Target at most 3 primary files per step; the packet total is larger because it is nine small slices.

- `crates/slicer-runtime/src/builtins/support_analysis_producer.rs` - role: AC-8 emission; expected change: per-region assignment minting.
- `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` - role: AC-1 target test stays green unmodified; may add a decline-entry observation if Ruling 1's structured record needs pinning.
- `crates/slicer-scheduler/src/validation.rs` (+ its test file) - role: G-21; expected change: family-scoped exemptions, positive+negative tests.
- `modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs` - role: G-23; expected change: real occupancy inputs + precondition marker.
- `resources/golden/benchy_tree_support_regression_{branch_count,endpoints}.txt` - role: rebless artifacts (via env gate only, never hand-edited).
- `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` - role: G-22; expected change: two `[config.schema]` entries.
- `docs/15_config_keys_reference.md` - role: generated-tables regen (via xtask, not hand-edited).
- `crates/slicer-runtime/tests/common/integrated_parity_harness.rs` - role: G-24; expected change: `assert_guest_freshness`.
- `crates/slicer-wasm-host/src/marshal/native.rs` (+ new test file) - role: view-seam identity test.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` (+ new test) - role: BASE fallback fix.
- Docs: `docs/specs/support-generation-remediation-plan.md`, `docs/adr/0059-*.md`, `docs/specs/support-parity-gap-register.md`, `docs/DEVIATION_LOG.md`, `docs/04_host_scheduler.md`, `docs/07_implementation_status.md`.

## Read-Only Context

Include ranges for files over 300 lines.

- `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` - lines 480–595 - purpose: AC-1 assertion shape (count {7,42}, byte-identical skeletons).
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` - ±40 lines around 857 - purpose: fallback branch context.
- `modules/core-modules/tree-support-planner/src/lib.rs` - DO NOT load; ~5.9k lines. Delegate symbol lookups (`candidate_family` at ~3814, `run_support_geometry_with_analysis` at ~1537).
- `OrcaSlicerDocumented/**` - delegated reads only (T1: verify existence by direct listing if needed).

## Out-of-Bounds Files

- `OrcaSlicerDocumented/...` - delegate; never load
- `target/`, `Cargo.lock`, generated code, vendored dependencies - never load
- `modules/core-modules/tree-support-planner/src/lib.rs` full-file loads - ranged/delegated only
- Planner/renderer algorithm files owned by 238b/238c (`tree-support/src/lib.rs`, `traditional-support/src/lib.rs`, `traditional-support-planner/src/lib.rs`) - behavior fixes out of scope here
- Other packets' directories under `docs/spec_packets/` - never modified

## Expected Sub-Agent Dispatches

- Question: confirm `FILL_CLAIM_IDS` contents and the exact exemption block shape in `validate_claim_conflicts`; scope: `crates/slicer-scheduler/src/validation.rs`; return: SNIPPETS ≤30 lines; purpose: Step 3 authoring reference.
- Question: locate the scheduler test binary/file that hosts claim-conflict validator tests today (for placing AC-N1/N2 tests correctly); scope: `crates/slicer-scheduler/tests/`; return: LOCATIONS ≤20; purpose: Steps 2–3.
- Question: SUMMARY of `docs/04_host_scheduler.md` "Validation Passes" section wording; return: SUMMARY ≤200 words; purpose: Step 8 doc edit.
- Question: re-derive highest `DEV-###` open row in `docs/DEVIATION_LOG.md` and highest `TASK-###` in `docs/07_implementation_status.md` immediately before those writes; scope: the two files; return: FACT 2 lines; purpose: ledger-fact re-derivation (Steps 5 and 9).
- Question: measure XY path length + deposited filament for PnP and Orca reference G-code files (parser script, not model reading); scope: `tmp/*.gcode`; return: FACT 4 numbers; purpose: Step 8 re-measurement.

## Data and Contract Notes

- IR/manifest contracts: `SupportAnalysisIR.family_assignments` gains rows for candidate-less regions — additive only; consumers (executor routing, `backfill_active_region_configs` pairing) must tolerate a region whose assignment exists without candidates; verify `structured_support_identity` integration test stays green.
- WIT boundary: none touched (no `.wit` edits this packet). The SDK view already carries the needed shape (`SupportFamilyAssignment { object_id, region_id, family_id }`).
- Determinism/scheduler constraints: the validator exemption is order-independent (set membership, not ordering); the per-region minting iterates RegionMap entries deterministically (BTreeMap-style keys as today) so serial/parallel runs stay identical (invariant 12).

## Locked Assumptions and Invariants

- Frozen golden tolerances: branch-count drift ≤ 10%, Hausdorff ≤ 0.5 mm — never widened (E3).
- The AC-8 test's count assertion is NOT weakened (plan Ruling 1).
- `support_threshold_angle` canonical semantics locked: min 0, max 90, default 30.0 (canonical `PrintConfig.cpp` coInt; recorded in-tree at `resolved_config.rs:968` macro doc).
- Family-scoped claim exemption applies to the GLOBAL conflict pass only; per-region conflict detection is unchanged. The `SupportPlanIR` / `SupportIR` multi-writer advisories are resolved by ORDERABILITY recognition (ADR-0059's host-aggregator aggregation edge), preserving the ADR's sole-writer model — no amendment deviation, no superseding ADR.
- check-literals violation count unchanged from inherited baseline (61 across 34 files, T10).

## Risks and Tradeoffs

- Per-region minting could double-count assignments if a region HAS candidates and the region-walk re-adds them — mitigate by keying the map insert (entry API) so each `(object, region)` lands exactly once; the executor-side consumer must see identical assignments for candidate-bearing regions as before.
- The G-21 write-conflict handling risks masking a future genuine dual-writer on `SupportPlanIR` — mitigated by implementing it as orderability recognition (aggregation edge = ordering, per ADR-0059's own clause) scoped to family-scoped support claim holders, plus the AC-N1 negative guard `genuine_write_conflict_still_rejected_after_aggregation_recognition`; a non-support dual-writer pair still conflicts.
- Tripwire rebless bakes current planner output as baseline; if the inputs change planner output materially, drift classification MUST precede regeneration (E3) or the golden loses meaning.
- Freshness assertion inside shared harness affects all integrated-parity suites (not just support): acceptable — it fails loudly only when artifacts are genuinely stale, which is the desired T4 posture; document in `docs/04_host_scheduler.md`? No — harness is test-common, note it in the harness doc comment instead.
- Deleting draft dirs removes their task-map crosswalks; git history is provenance (plan §10) — remediation rows record the absorption mapping.

## Context Cost Estimate

- Aggregate: `M`
- Largest step: `M` (Step 8 green gate — dispatch-heavy, bounded returns)
- Highest-risk dispatch and required return format: deviation/TASK ledger re-derivation (FACT, must be re-run at write time, never trusted from this document)

## Open Questions

- `[FWD]→237+`: should the family-scoped global-claim exemption generalize to future family-scoped claim groups beyond support (pattern is now established twice: fill-role claims, support families)? Route to the next packet adding a claim family; do not generalize preemptively in this packet.
- `[FWD]→240`: the signed-index migration (u32→i32) touches `GlobalLayer.index` etc.; this packet's per-region minting adds no index-type changes, but 240 must re-run AC-1 after the migration.
- `[BLOCK]`: none.
