---
status: draft
packet: "236-support-stabilization"
spec: support-stabilization
depends_on: none
task_ids:
  - TASK-344
  - TASK-345
  - TASK-346
  - TASK-347
  - TASK-348
  - TASK-349
  - TASK-350
  - TASK-351
  - TASK-352
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
---

# Packet Contract: 236-support-stabilization

## Goal

Make the support-families branch contract-stable and fully green: per-region AC-8 family assignment lands, the startup validator stops flagging expected post-221 multi-holder support claims, the only tree-geometry golden exercises real collision/avoidance inputs, `support_threshold_angle` gains declared bounds with regenerated config docs, the integrated-parity harnesses assert guest freshness before comparing, the native/wasm view seam and paint BASE fallback get pinned, drafts 215–218 are deleted, ADR-0059 is accepted, and post-fix parity ratios are measured.

## Scope Boundaries

This packet is a stabilization sweep over nine independently-small surfaces already routed to it by `docs/specs/support-families-anchored-entities-plan.md` §12 (brief "236-support-stabilization") and gap-register rows G-21/G-22/G-23/G-24. It changes host analysis emission, scheduler validation, one module manifest, test harnesses/fixtures, and documentation — it does NOT touch planner or renderer algorithm fidelity (238b/238c), the `needs_support` signal (237), or raft geometry (240). Full lists live in `requirements.md`; ACs are authoritative here.

## Prerequisites and Blockers

- Depends on: none (first packet of the support-families completion queue).
- Unblocks: 237-support-analysis-parity, 238a-support-pattern-config-keys, 240-support-raft (all depend on queue row #1).
- Activation blockers: none. Status stays `draft` until `/spec-review --preflight` passes and the Human Validation Gate below is signed.

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs. Every `--exact` name below is either verified to exist today or is authored by this packet in the cited step (documented in `requirements.md` §In Scope).

- **AC-1. Given** the multi-region fixture in `crates/slicer-runtime/tests/executor/prepass_support_geometry_layer_plan_tdd.rs` (`multi_region_map`: regions 7 and 42 for `obj-multi`), **when** `commit_support_analysis_builtin` (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) mints `family_assignments`, **then** every RegionMap region receives exactly one attributed assignment (minted per region, not per candidate; regions without candidates keep their config-resolved family via the same `config_for_region_smallest_chain` → `support_family` two-stage lookup) and `planner_emits_one_entry_per_region_in_region_map` passes with its count assertion unchanged: 2 entries per emitted layer, region_ids {7, 42}, byte-identical skeletons. | `cargo test -p slicer-runtime --test executor -- prepass_support_geometry_layer_plan_tdd::planner_emits_one_entry_per_region_in_region_map --exact`
- **AC-2. Given** a full-directory module set whose support claims are family-scoped post-221 (`support-generator`, `support-planner`, `support-family:traditional`, `support-family:tree`), **when** `validate_startup_dag` (`crates/slicer-scheduler/src/validation.rs`) runs its `GlobalClaimConflicts` and `WriteConflicts` passes, **then** it reports zero `ClaimConflict` entries for those four claims, and zero `WriteConflict` entries for `SupportPlanIR` / `SupportIR` pairs whose modules both hold family-scoped support claims — such pairs are recognized as ORDERABLE-BY-AGGREGATION per ADR-0059's own write-topology clause (family planners emit family-scoped entries; the host aggregator is the sole writer of the aggregated plan), not blanket-exempted — while the per-region conflict pass is unchanged. | `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::family_scoped_support_claims_do_not_conflict_globally --exact`
- **AC-3. Given** the reblessed goldens at `resources/golden/benchy_tree_support_regression_branch_count.txt` and `resources/golden/benchy_tree_support_regression_endpoints.txt` (workspace root — path corrected from the plan; see `design.md` §Plan Corrections), **when** `benchy_tree_support_regression_tripwire` runs WITHOUT the regeneration environment variable, **then** it passes under the frozen tolerances (branch count within ±10%, Hausdorff ≤ 0.5 mm — E3, never widened). | `cargo test -p tree-support-planner -- benchy_tree_support_regression_tripwire --exact`
- **AC-4. Given** the strengthened tripwire fixture, **when** `benchy_tree_support_regression_tripwire` builds its `SupportGeometryView`, **then** the view carries non-empty occupancy entries so the collision and avoidance ladders are exercised, and the source carries the packet's precondition marker `G-23 fixture precondition` immediately before that assertion. | `rg -q 'G-23 fixture precondition' modules/core-modules/tree-support-planner/tests/orca_parity_tdd.rs && echo PASS || echo FAIL`
- **AC-5. Given** the `traditional-support-planner` manifest (`modules/core-modules/traditional-support-planner/traditional-support-planner.toml`), **when** its `[config.schema]` is read, **then** both `support_threshold_angle` and the legacy alias `support_overhang_angle` are declared with `min = 0`, `max = 90`, `default = 30.0`, and the generated config docs are regenerated in the same commit (`cargo xtask gen-config-docs --check` green). These are declaration-only entries: no planner src reads either key today (contact detection consumes the host-typed `support_threshold_angle` field); declaring them arms the scheduler bounds index (`ConfigBoundsIndex` aggregates manifest `[min, max]` exclusively), which is what AC-N2 exercises. | `rg -q '^\[config\.schema\.support_threshold_angle\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && rg -q '^\[config\.schema\.support_overhang_angle\]' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && cargo xtask gen-config-docs --check`
- **AC-6. Given** the shared integrated-parity harness (`crates/slicer-runtime/tests/common/integrated_parity_harness.rs`), **when** any of the four support integrated-parity comparisons runs (`integrated_parity_support_planner_tdd.rs`, `integrated_parity_tree_support_tdd.rs`, `integrated_parity_traditional_support_tdd.rs`, `integrated_parity_support_surface_ironing_tdd.rs` under `crates/slicer-runtime/tests/contract/`), **then** guest freshness is asserted (via `assert_guest_freshness` inside `run_integrated_parity`, or a direct call in any harness that bypasses it) before the native/wasm comparison executes. | `rg -q 'assert_guest_freshness' crates/slicer-runtime/tests/common/integrated_parity_harness.rs && echo PASS || echo FAIL`
- **AC-7. Given** fresh guest artifacts, **when** the support-planner integrated-parity harness runs, **then** `integrated_parity_support_planner_native_matches_wasm` passes with the freshness assertion active. | `cargo test -p slicer-runtime --test contract -- integrated_parity_support_planner_tdd::integrated_parity_support_planner_native_matches_wasm --exact`
- **AC-8. Given** one representative `(layer, object)` input, **when** the native leg builds its layer view via `build_native_layer_request` (`crates/slicer-wasm-host/src/marshal/native.rs`) and the wasm leg builds its view via the `dispatch_layer_call` projection, **then** the new test `native_and_wasm_layer_views_are_field_identical` asserts the two views field-identical (T9 leg-skew guard; hit 3× in the 224 tail). | `cargo test -p slicer-wasm-host --test contract -- view_seam_identity_tdd::native_and_wasm_layer_views_are_field_identical --exact`
- **AC-9. Given** a painted layer whose RegionMap carries no BASE entry for that layer, **when** `execute_paint_segmentation` (`crates/slicer-core/src/algos/paint_segmentation/mod.rs`) takes the `matching_base.is_empty()` fallback, **then** the fallback derives BASE polygons from the object's OWN contours rather than whole-layer all-object contours, pinned by the new test `paint_base_fallback_uses_own_object_contours` in the new standalone test binary `paint_segmentation_base_fallback_tdd` (slicer-core has no test aggregator; binaries are auto-discovered, gated `#![cfg(feature = "host-algos")]`). | `cargo test -p slicer-core --features host-algos --test paint_segmentation_base_fallback_tdd -- paint_base_fallback_uses_own_object_contours --exact`
- **AC-10. Given** the superseded draft packets, **when** the deletions land, **then** `docs/spec_packets/215-raft-geometry/`, `docs/spec_packets/216-support-interface-layers/`, `docs/spec_packets/217-support-type-variants/`, and `docs/spec_packets/218-support-gcode-e2e/` are absent and rows 3–6 of `docs/specs/support-generation-remediation-plan.md` no longer carry a live `| generated | docs/spec_packets/21xx-… |` destination. | `test ! -d docs/spec_packets/215-raft-geometry && test ! -d docs/spec_packets/216-support-interface-layers && test ! -d docs/spec_packets/217-support-type-variants && test ! -d docs/spec_packets/218-support-gcode-e2e && ! rg -q '\| generated \| docs/spec_packets/21[5678]-' docs/specs/support-generation-remediation-plan.md && echo PASS || echo FAIL`
- **AC-11. Given** ADR-0059 (`docs/adr/0059-support-families-and-anchored-entities.md`, currently `Status: proposed`), **when** the packet accepts it, **then** the status field reads `accepted` and an `## Amendments` section records the Ruling-1 per-region family-assignment decision (plan §3 Ruling 1, §5 amendment). | `rg -q '^Status: accepted' docs/adr/0059-support-families-and-anchored-entities.md && rg -q '^## Amendments' docs/adr/0059-support-families-and-anchored-entities.md && echo PASS || echo FAIL`
- **AC-12. Given** fresh PnP tree/traditional G-code of `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` and the Orca reference G-code under `tmp/` (gitignored — verify by direct `test -s`, T1), **when** XY path-length and deposited-material are re-measured after this packet's behavior fixes (post Ruling-1), **then** `docs/specs/support-parity-gap-register.md` carries the dated note with the exact marker `packet-236 re-measurement`, superseding the stale pre-AC-1-fix figures (which must never be requoted). | `rg -q 'packet-236 re-measurement' docs/specs/support-parity-gap-register.md && echo PASS || echo FAIL`

## Negative Test Cases

- **AC-N1. Given** two modules genuinely holding the same non-support claim globally (e.g. two `seam-placer` holders), **when** `validate_startup_dag` runs, **then** the `ClaimConflict` is still reported — and, symmetrically, two modules writing the same IR field where NEITHER pair relation is covered by an ADR-recognized aggregation/orderability rule still produce a `WriteConflict` (the family-scoped exemptions must not silence real conflicts; G-21 validator-contract change rejection cases). | `cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_claim_conflict_still_rejected_after_family_exemption --exact && cargo test -p slicer-scheduler --test scheduler_unit -- stage_canon_seam_support_tdd::genuine_write_conflict_still_rejected_after_aggregation_recognition --exact`
- **AC-N2. Given** a config that sets `support_threshold_angle` to 200 degrees, **when** config resolution runs, **then** resolution fails with `ConfigResolutionError::OutOfRange` instead of silently resolving to the in-code default (G-22 bounds enforcement rejection case; mechanism: TASK-182 `ConfigBoundsIndex`, test home `crates/slicer-scheduler/tests/integration/config_bounds_enforcement_tdd.rs`). | `cargo test -p slicer-scheduler --test scheduler_integration -- config_bounds_enforcement_tdd::out_of_range_support_threshold_angle_is_rejected --exact`

## Verification

Packet-level gates only; the full matrix belongs in `requirements.md`. Broad-run discipline per E5/T3: totals only from `--no-fail-fast`, read `target/test-output.log`, never re-run for more output.

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask test --summary --workspace -- --no-fail-fast` (the plan §12 green gate; requires `cargo xtask build-guests --check` freshness first — exit 0 fresh / 1 stale / 3 infra, never grep for `STALE:`)

Recorded, not gating: `cargo xtask check-literals --report` — the inherited G-15 debt (61 violations across 34 files) is neither this packet's blocker nor its credit; the count must not increase (T10).

## Authoritative Docs

- `docs/specs/support-families-anchored-entities-plan.md` - 755 lines; delegate §-range summaries (§3 Ruling 1, §12 brief 236, §13 traps T1–T11, §14 authoring rules). Never load whole.
- `docs/specs/support-parity-gap-register.md` - rows G-21, G-22, G-23, G-24; direct range read (destinations already routed to this packet).
- `docs/adr/0059-support-families-and-anchored-entities.md` - short; direct read.
- `AGENTS.md` - direct read (test discipline, guest WASM staleness, coordinate hazard, literal gate).

## Doc Impact Statement (Required)

Specific same-packet doc edits, each with its verification grep:

- `docs/15_config_keys_reference.md` generated tables (regenerated after the G-22 manifest change) - `cargo xtask gen-config-docs --check` (exit 0).
- `docs/adr/0059-support-families-and-anchored-entities.md` status + amendments - `rg -q '^Status: accepted' docs/adr/0059-support-families-and-anchored-entities.md && rg -q '^## Amendments' docs/adr/0059-support-families-and-anchored-entities.md && echo PASS || echo FAIL`
- `docs/specs/support-generation-remediation-plan.md` rows 3–6 disposition - `! rg -q '\| generated \| docs/spec_packets/21[5678]-' docs/specs/support-generation-remediation-plan.md && echo PASS || echo FAIL`
- `docs/specs/support-parity-gap-register.md` re-measurement note - `rg -q 'packet-236 re-measurement' docs/specs/support-parity-gap-register.md && echo PASS || echo FAIL`
- `docs/DEVIATION_LOG.md` G-23 rebless justification row (E3; re-derive the next free DEV-### at write time) - `cargo xtask check-deviations` (exit 0).
- `docs/04_host_scheduler.md` "Validation Passes" section (family-scoped exemption documented) - `rg -q 'family-scoped' docs/04_host_scheduler.md && echo PASS || echo FAIL`
- `docs/07_implementation_status.md` TASK-344..TASK-352 rows (registered via the Step 9 worker dispatch, packet-232 precedent) - `rg -q 'TASK-344' docs/07_implementation_status.md && rg -q 'TASK-352' docs/07_implementation_status.md && echo PASS || echo FAIL`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `support_threshold_angle` declaration (coInt, min 0, max 90, default 30): delegated confirmation only. The in-tree doc comment on the `support_threshold_angle` macro line in `crates/slicer-ir/src/resolved_config.rs` already records these values; read the canonical file only if the G-22 declared range is disputed.

Parity comparisons in this packet run against the human-regenerated Orca reference G-code under `tmp/` (`tmp/SupportTest_Tree_Orca.gcode`, `tmp/SupportTest_Normal_Orca.gcode`) — build artifacts, not `OrcaSlicerDocumented/` reads. This packet cites no canonical algorithm source; planner/renderer fidelity work belongs to 238b/238c.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

## Human Validation Gate

Per plan §8, blocking: this packet may not flip to `status: implemented` without a signed sign-off line below.

Artifact-producing commands (record the exact invocation used, including the profile flag resolved via `cargo run --bin pnp_cli -- slice --help` at gate time, into `docs/spec_packets/236-support-stabilization/evidence/human-gate.md`):

- Tree: slice `crates/slicer-runtime/tests/fixtures/support-family/SupportTest.stl` with the matched tree profile `tmp/support-family-config-tree-matched.json` → `tmp/p236-tree.gcode` (T1: `tmp/` is gitignored; verify inputs by direct `test -s`, regenerate if missing).
- Traditional: same fixture with `tmp/support-family-config-normal-matched.json` → `tmp/p236-normal.gcode`.
- Visual-debug bundle for this packet's boundary (support-plan emission per region, post-AC-8): one bundle per family from the same fixture, request JSONs recorded in the evidence file.

Inspection checklist (each item named verdict in the evidence file — E2: inspection is satisfied by the written checklist, never by a test):

- Termination: every support branch reaches the plate or the model beneath the overhang; no floating endpoints.
- Coverage: overhang regions of the fixture carry support beneath them in both families.
- Collision freedom: no support body intersects model geometry in the renders.
- Interfaces: `;TYPE:Support interface` bands present where configured (`support_interface_top_layers = 2`).
- Block counts: `;TYPE:Support` / `;TYPE:Support interface` block counts vs the Orca references (`tmp/SupportTest_Tree_Orca.gcode`, `tmp/SupportTest_Normal_Orca.gcode`) recorded as measured deltas, not asserted equal (G-18 divergence is 238c-owned).

Artifact locations: `tmp/p236-tree.gcode`, `tmp/p236-normal.gcode`, visual-debug PNG bundles under `tmp/`, evidence file `docs/spec_packets/236-support-stabilization/evidence/human-gate.md`.

- Sign-off (human, blocking): pending — record date + verdict here. Until signed, the packet stays `draft`/in-flight regardless of green gates.
