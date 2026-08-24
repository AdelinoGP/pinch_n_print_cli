# Requirements: 236-support-stabilization

## Packet Metadata

- Grouped task IDs: `TASK-344` through `TASK-352` (allocated fresh; next free after the re-derived high-water mark `TASK-343` in `docs/07_implementation_status.md`; TASK-324..328 never reused)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M` (largest step M — Step 8's green gate is dispatch-heavy, not read-heavy)

## Problem Statement

The support-families branch (`parity/support-planners-clean`) carries a deliberately red AC-8 test, a validator that flags expected post-221 claim topology as permanent noise (G-21), a golden that proves nothing about collision or avoidance because its fixture feeds empty occupancy (G-23), an unbounded host config key whose doc reference was left stale by a past manifest deletion (G-22), and parity harnesses that can report spurious geometry divergences from guest staleness (G-24). Two latent correctness hazards remain unpinned: the native/wasm layer-view construction seam that silently rendered inputs three times during packet 224 (T9), and `execute_paint_segmentation`'s BASE fallback built from whole-layer all-object contours. The queue also still lists four never-implemented draft packets and a `proposed` ADR that this queue's Ruling 1 amends. These are one coherent slice: each item is small, independently verifiable, and every later packet in the completion queue (237, 238a/b/c, 240) builds on a branch that must be green and honestly instrumented first.

## In Scope

Authoritative full scope. Steps map to `implementation-plan.md`.

1. **AC-8 per-region ruling** (plan §3 Ruling 1, §12): change `commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`, per-candidate mint loop at lines ~253–279) so `family_assignments` are minted per RegionMap region; regions with no candidate receive their structured assignment via the same two-stage lookup (`config_for_region_smallest_chain` → `support_family` → `"traditional"` fallback). The test `planner_emits_one_entry_per_region_in_region_map` (`crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs:503`) keeps its count assertion unchanged. The disproved mesh-path-gate hypothesis (plan T11) must NOT be re-attempted.
2. **G-21 validator contract**: extend `validate_startup_dag` (`crates/slicer-scheduler/src/validation.rs`; passes `GlobalClaimConflicts` line ~420 / `WriteConflicts` line ~709; existing fill-role multi-holder exemption precedent at line ~553 via `FILL_CLAIM_IDS`) so family-scoped support CLAIMS (`support-generator`, `support-planner`, `support-family:traditional`, `support-family:tree`) are exempt from global-claim-conflict advisories, and so the known family-planner pairs writing `SupportPlanIR` / `SupportIR` are recognized as ORDERABLE-BY-AGGREGATION per ADR-0059's own write-topology clause (family planners emit only family-scoped entries; "the host aggregator is the sole writer of the aggregated `SupportPlanIR`") — the aggregation edge IS the ordering, so the pair is orderable, not exempt. This preserves ADR-0059's host-aggregator sole-writer model exactly; no ADR amendment or deviation is required, and genuine conflicts stay reported (AC-N1).
3. **G-23 tripwire strengthening + classified rebless**: give `benchy_tree_support_regression_tripwire` (`modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs:400`) real collision/avoidance inputs — non-empty occupancy in the `SupportGeometryView { entries: vec![] }` at line ~421 — classify the drift (E3), record the justification in `docs/DEVIATION_LOG.md`, then rebless via `SUPPORT_PLANNER_REGEN_GOLDEN=1`. Golden files live at workspace-root `resources/golden/benchy_tree_support_regression_{branch_count,endpoints}.txt` (see `design.md` §Plan Corrections). Frozen tolerances stand: ±10% branch count, Hausdorff ≤ 0.5 mm.
4. **G-22 bounds + doc regen**: declare `support_threshold_angle` AND legacy alias `support_overhang_angle` with `min = 0`, `max = 90`, `default = 30.0` in `[config.schema]` of `modules/core-modules/traditional-support-planner/traditional-support-planner.toml` (schema block starts at line 27); run `cargo xtask gen-config-docs` in the same commit (T8 same-commit rule; the deletion commit `4d1848eb` left `docs/15_config_keys_reference.md` stale). Negative case: out-of-range value rejected with `ConfigResolutionError::OutOfRange` (AC-N2).
5. **G-24 freshness asserts**: add `assert_guest_freshness` to the shared harness `crates/slicer-runtime/tests/common/integrated_parity_harness.rs` and wire it through `run_integrated_parity` so all four support integrated-parity contract tests (`integrated_parity_support_planner_tdd.rs`, `integrated_parity_tree_support_tdd.rs`, `integrated_parity_traditional_support_tdd.rs`, `integrated_parity_support_surface_ironing_tdd.rs` under `crates/slicer-runtime/tests/contract/`) assert guest freshness before comparing (E4; staleness presents as a count divergence like `native=128 wasm=126`, not an instantiation error).
6. **Native/wasm view-seam guard**: pin the two leg-construction paths field-identical — native via `build_native_layer_request` (`crates/slicer-wasm-host/src/marshal/native.rs:113`), wasm via the `dispatch_layer_call` projection — with new test `native_and_wasm_layer_views_are_field_identical` (AC-8), or refactor to one shared construction path if the implementer finds it smaller; the test is mandatory either way (T9 hit 3×: commits `85f1f889`, `ddf9dffe`, `with_slice_ir`).
7. **Paint BASE fallback correction**: fix `matching_base.is_empty()` fallback in `execute_paint_segmentation` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs:857`) so single-object layers derive BASE polygons from the object's own contours instead of whole-layer all-object contours; pinned by new test `paint_base_fallback_uses_own_object_contours` (AC-9). E6 applies: slicer-core tests need `--features host-algos`.
8. **Deletions, remediation rows, ADR acceptance**: delete `docs/spec_packets/{215-raft-geometry,216-support-interface-layers,217-support-type-variants,218-support-gcode-e2e}/` (git history is provenance); update rows 3–6 of `docs/specs/support-generation-remediation-plan.md` (lines 61–64, the only live references — verified) to point at their absorbing packets; flip ADR-0059 `proposed` → `accepted` with the Ruling-1 amendment note (AC-10, AC-11).
9. **Re-measurement + green gate + human gate**: measure post-fix tree/traditional XY path-length and deposited-material vs the Orca references under `tmp/`; record dated figures in `docs/specs/support-parity-gap-register.md` marked `packet-236 re-measurement` (supersedes stale pre-AC-1-fix figures — never requoted); run the full green gate (E5/T3: `cargo xtask test --summary --workspace -- --no-fail-fast`, clippy `-D warnings`, check-literals count unchanged); produce Human Validation Gate artifacts.

## Out of Scope

- Planner/renderer algorithm fidelity (top-Z gap, smoothing, role coexistence, circle fidelity, hollow walls, density scaling) — 238b/238c.
- The real `needs_support` eligibility signal, enforcers-under-auto routing, five missing `detect_overhangs` steps — 237.
- Raft geometry, signed-index migration, raft keys — 240.
- AGG rasterizer port and the `support_area_rasterizer` knob — 241.
- Pattern/expansion/bottom-z/line-width key declarations beyond G-22 — 238a.
- G-18 interface block counts, DEV-129/DEV-145 corrections — 238c.
- G-14 `ERR_MALFORMED_LAYER_MARKER` noise and the G-15 literal debt (61 violations across 34 files): pre-existing, recorded only — count unchanged, no credit (T10).
- Widening any frozen golden tolerance (E3 prohibits it; widen the fixture margin or record a deviation instead).

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - 755 lines; delegate §-range summaries. Binding sections: §3 rulings, §7 evidence standards E1–E9, §8 human validation gate, §12 brief "236", §13 traps T1–T11, §14 authoring rules.
- `docs/specs/support-parity-gap-register.md` - direct range read of rows G-21..G-24 (~40-line window).
- `docs/specs/support-generation-remediation-plan.md` - lines 55–70 only (rows 3–6 context).
- `docs/adr/0059-support-families-and-anchored-entities.md` - short; direct read.
- `docs/04_host_scheduler.md` - delegate SUMMARY of "Validation Passes" section before editing it.
- `AGENTS.md` - direct read (test discipline, guest staleness exit codes, coordinate hazard).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `support_threshold_angle` declaration (coInt, min 0, max 90, default 30): delegated confirmation only if the G-22 declared range is disputed; the in-tree macro doc comment in `crates/slicer-ir/src/resolved_config.rs` already records these values.

Parity comparisons in this packet run against the human-regenerated Orca reference G-code under `tmp/` (`tmp/SupportTest_Tree_Orca.gcode`, `tmp/SupportTest_Normal_Orca.gcode`, gitignored — verify existence by direct listing per trap T1, regenerate before relying on them). No canonical algorithm source is cited by this packet's code changes.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (per-region minting), `AC-2` (validator silence on family-scoped claims), `AC-3`+`AC-4` (tripwire reblessed on real inputs), `AC-5` (declared bounds + regenerated docs), `AC-6`+`AC-7` (freshness asserted and effective), `AC-8` (view-seam identity), `AC-9` (paint BASE fallback), `AC-10`+`AC-11` (deletions, remediation rows, ADR accepted), `AC-12` (re-measurement recorded). Measurable refinements: AC-1 asserts region_ids {7,42} and byte-identical skeletons; AC-3 pins tolerances ±10% / ≤0.5 mm without widening.
- Negative: `AC-N1` (genuine claim conflict AND genuine non-aggregation write conflict still rejected after the family exemption / aggregation recognition), `AC-N2` (out-of-range `support_threshold_angle` → `ConfigResolutionError::OutOfRange`).
- Cross-packet impact: 237 inherits the per-region assignment semantics; 238a inherits the declared-key + regen-docs pattern; 238b/238c inherit the strengthened tripwire as their regression net; 242 inherits the freshness-assert harness pattern.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only gate commands. Every command returns a small parseable result; every named test exists today or is authored by the step cited in §In Scope (invariant 16: zero-match filters forbidden).

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-runtime --test executor -- prepass_support_geometry_layer_plan_tdd::planner_emits_one_entry_per_region_in_region_map --exact` | AC-1 | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::family_scoped_support_claims_do_not_conflict_globally --exact && cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_claim_conflict_still_rejected_after_family_exemption --exact && cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_write_conflict_still_rejected_after_aggregation_recognition --exact` | AC-2 + AC-N1 (one TESTNAME per invocation) | FACT pass/fail all three |
| `cargo test -p slicer-scheduler --test scheduler_integration -- config_bounds_enforcement_tdd::out_of_range_support_threshold_angle_is_rejected --exact` | AC-N2 | FACT pass/fail |
| `cargo test -p tree-support-planner -- benchy_tree_support_regression_tripwire --exact` | AC-3 | FACT pass/fail |
| `rg -q 'G-23 fixture precondition' modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs && echo PASS \|\| echo FAIL` | AC-4 | FACT PASS/FAIL |
| `cargo xtask gen-config-docs --check` | AC-5 docs regen | FACT pass/fail (exit code) |
| `cargo test -p slicer-runtime --test contract -- integrated_parity_support_planner_tdd::integrated_parity_support_planner_native_matches_wasm --exact` | AC-7 (+AC-6 via shared harness grep) | FACT pass/fail |
| `cargo xtask build-guests --check; echo "exit=$?"` | Guest freshness before any attribution (T4) | FACT exit=0/1/3 |
| `cargo test -p slicer-wasm-host --test contract -- view_seam_identity_tdd::native_and_wasm_layer_views_are_field_identical --exact` | AC-8 | FACT pass/fail |
| `cargo test -p slicer-core --features host-algos --test paint_segmentation_base_fallback_tdd -- paint_base_fallback_uses_own_object_contours --exact` | AC-9 (E6 feature flag mandatory; standalone auto-discovered binary, no aggregator) | FACT pass/fail |
| `test ! -d docs/spec_packets/215-raft-geometry && test ! -d docs/spec_packets/216-support-interface-layers && test ! -d docs/spec_packets/217-support-type-variants && test ! -d docs/spec_packets/218-support-gcode-e2e && echo PASS \|\| echo FAIL` | AC-10 deletions | FACT PASS/FAIL |
| `rg -q '^Status: accepted' docs/adr/0059-support-families-and-anchored-entities.md && rg -q '^## Amendments' docs/adr/0059-support-families-and-anchored-entities.md && echo PASS \|\| echo FAIL` | AC-11 | FACT PASS/FAIL |
| `rg -q 'packet-236 re-measurement' docs/specs/support-parity-gap-register.md && echo PASS \|\| echo FAIL` | AC-12 | FACT PASS/FAIL |
| `cargo check --workspace --all-targets` | compile gate incl. test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | lint gate | FACT pass/fail |
| `cargo xtask check-literals --report` | count unchanged vs inherited 61 (T10) | FACT: violation count |
| `cargo xtask check-deviations` | deviation-log consistency after rebless row | FACT pass/fail |
| `cargo xtask test --summary --workspace -- --no-fail-fast` | packet green gate (Step 9 only) | FACT PASS/FAIL from summary digest |

## Step Completion Expectations

- Order matters: Step 2 (negative fixture, red) precedes Step 3 (validator change, green); the G-23 drift classification note lands BEFORE the rebless command runs (E3 forbids silent regeneration); the manifest edit and `gen-config-docs` regeneration land in the SAME commit (T8); deletions and remediation-row updates are one commit (plan §10); the green gate runs only after all behavior steps, with `cargo xtask build-guests --check` first (exit 0 fresh / 1 stale → rebuild / 3 infra error — never grep `STALE:`).
- Any test failure attributed to guests, dispatch, or parity MUST be preceded by a fresh `cargo xtask build-guests --check` exit-0 result in the same session (AGENTS.md rule).
- Broad-run output is read from `target/test-output.log`, never re-run for more output (E5/T3).

## Context Discipline Notes

- `modules/core-modules/tree-support-planner/src/lib.rs` is ~5.9k lines — ranged reads only; never load whole. The tripwire work should need NO planner source edits.
- `crates/slicer-core/src/algos/paint_segmentation/mod.rs` is large — locate `execute_paint_segmentation` and open a ±40-line window around the fallback (line ~857 verified).
- `tmp/` artifacts (matched profiles, Orca references) are gitignored — glob tools miss them (T1); verify by direct `ls`.
