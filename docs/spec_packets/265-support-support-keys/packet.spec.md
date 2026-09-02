---
status: draft
packet: support-support-keys
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/20-author-packet-p13-support-support-support-planner.md (wayfinder map: Close the OrcaSlicer FFF feature gap — packet P13)
context_cost_estimate: L
---

# Packet Contract: support-support-keys

## Goal

Make ten of ticket 20's twelve Support/Support keys drive real behaviour: build the five missing decision points — `enforce_support_layers` (forced contacts on the lowest N layers), `support_critical_regions_only` (drop ordinary overhangs, keep cantilevers and sharp tails), `support_remove_small_overhang` (canonical's erode-and-measure cluster filter), `support_bottom_z_distance` (an object-supported bottom gap symmetric with the live top gap), and `support_object_first_layer_gap` (a layer-0 override of the XY clearance) — and prove the five already-live keys behave at non-default values. `raft_first_layer_expansion` and `support_style` are returned to the queue, unimplemented, with their owners named.

## Scope Boundaries

The packet edits `slicer_core::algos::overhang_annotation` (two new `SupportContactParams` fields plus two new filter stages in `detect_support_contacts_with_annotations`), `slicer_runtime::builtins::support_analysis_producer`'s `resolve_contact_params` (three fields sourced from config instead of hardcoded neutrals), and both support planners (`traditional-support-planner`, `tree-support-planner`) for the bottom-Z gap and the first-layer XY override, plus their two manifests. It adds no module, no WIT interface, no IR schema field, and no `ResolvedConfig` field — every host key it consumes is already declared in `docs/config/host-keys.toml` and already carried on `ResolvedConfig`. It does **not** implement raft geometry (packet `240-support-raft` owns every raft key including `raft_first_layer_expansion`), does not port the organic tree engine or resolve the `support_style` manifest-type inconsistency (sibling-plan row 7, TASK-441), and touches neither `ORCA_CONFIG_PADDING` nor any CONFIG_BLOCK twin.

## Prerequisites and Blockers

- Depends on: wayfinder ticket 06 (packet numbering — number 265 re-derived from disk at authoring time); ticket 05 (packet-list P13 membership); ticket 04 (tier rubric — Tier **B** re-derived in `design.md` § Tier Derivation, was Tier A). Packet `238a-support-pattern-config-keys` (`status: implemented`) is a hard predecessor for `support_bottom_z_distance`: it made the key a real host-transported, bounds-enforced value and its `design.md` explicitly deferred "planner/renderer semantics that consume it" to later packets. This packet supplies those semantics.
- Ordering, not gating: packet `240-support-raft` owns `raft_first_layer_expansion` and appends raft tables to the same `tree-support-planner.toml`; this packet's two net-new tables there are same-manifest append churn.
- Activation blockers: none. No `[BLOCK]` is open — see `design.md` § Open Questions.

## Acceptance Criteria

Every criterion below asserts a behaviour change at a **non-default** value. Default-path identity appears only as an additional guard (AC-N4), never as any key's sole evidence.

- **AC-1 (`enforce_support_layers`, class b).** Given a model whose lowest three layers carry no angle-detected overhang, when the run config sets `enforce_support_layers = 3` (canonical default `0`), then `resolve_contact_params` yields a `SupportContactParams` whose `enforce_support_layers` field is `3`, and `detect_support_contacts_with_annotations` returns non-empty contacts at `layer_id` 0, 1 and 2 and empty contacts at `layer_id` 3 — whereas at the default `0` all four are empty. | `mkdir -p target && cargo test -p slicer-runtime --lib resolve_contact_params_sources_enforce_support_layers 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-2 (`support_critical_regions_only`, class b).** Given a layer carrying one ordinary angle-detected overhang and one cantilever surface (span beyond the canonical cantilever threshold), when `support_critical_regions_only = true` (canonical default `false`), then the returned `SupportContactAnnotations.contacts` contains the cantilever geometry and not the ordinary overhang; at `false` the identical input returns both. | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_critical_and_small_overhang_tdd critical_regions_only_keeps_only_cantilevers_and_sharp_tails 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-3 (`support_remove_small_overhang`, class b).** Given a layer with one overhang cluster whose eroded bounding box is narrower than `2 * external_perimeter_width_mm` on X, when `support_remove_small_overhang = false` (canonical default `true`), then the cluster survives into `contacts`; at the default `true` the identical input erases it, so the two runs differ in contact count. | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_critical_and_small_overhang_tdd remove_small_overhang_false_keeps_the_narrow_cluster 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-4 (`support_bottom_z_distance`, class b).** Given a support column whose descent terminates on a model surface (`model_termination_layer` resolves to `Some`), when `support_bottom_z_distance = 0.6` (canonical default `0.2`), then the planner's lowest emitted support layer index for that column is strictly greater than at `0.2`; and a column terminating on the build plate (`model_termination_layer` is `None`) emits from layer 0 at both values. | `mkdir -p target && cargo test -p traditional-support-planner --test support_gap_keys_tdd bottom_z_distance_raises_only_model_terminated_columns 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-5 (`support_object_first_layer_gap`, traditional family, class b).** Given a support body overlapping object occupancy on layer 0 and on layer 1, when `support_object_first_layer_gap = 1.0` (canonical default `0.2`) and `support_object_xy_distance = 0.35`, then the layer-0 trim clearance is `1.0` mm while the layer-1 trim clearance stays `0.35` mm, producing a strictly smaller layer-0 support area than a run with the first-layer gap at `0.35`. | `mkdir -p target && cargo test -p traditional-support-planner --test support_gap_keys_tdd first_layer_gap_overrides_xy_distance_on_layer_zero_only 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-6 (`support_object_first_layer_gap`, tree family, class b).** Given the tree planner's model-collision construction, when `support_object_first_layer_gap = 1.0` and the object layer index is 0, then the inflation distance handed to `inflate_model_occupancy` is `1.0` mm rather than `support_object_xy_distance`, and remains `support_object_xy_distance` for every object layer index above 0. | `mkdir -p target && cargo test -p tree-support-planner --test support_gap_keys_tdd first_layer_gap_applies_to_object_layer_zero_only 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-7 (`support_expansion`, class a).** Given a detected contact region, when `support_expansion = 0.5` (canonical default `0.0`), then the returned contact area is strictly greater than the area returned at `0.0` for the identical input. | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd expansion 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-8 (`support_threshold_angle`, class a).** Given a fixed overhang geometry that is detected at the canonical default `30`, when `support_threshold_angle = 60`, then the detected contact set is strictly smaller — the shallower overhang no longer qualifies. | `mkdir -p target && cargo test -p slicer-core --features host-algos --test support_overhang_detection_tdd threshold_angle 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-9 (`support_threshold_overlap`, class a).** Given `support_threshold_angle = 0` (canonical's overlap branch), when `support_threshold_overlap = "100%"` (canonical default `50%`), then `resolve_contact_params` yields `threshold_overlap_mm` equal to `external_perimeter_width_mm` rather than half of it, changing the lower-layer offset. | `mkdir -p target && cargo test -p slicer-runtime --lib resolve_contact_params_uses_typed_threshold_overlap_percent_and_literal 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-10 (`support_object_xy_distance`, class a).** Given a support body overlapping object occupancy, when `support_object_xy_distance = 1.0` (canonical default `0.35`), then the trimmed support area on every layer above 0 is strictly smaller than at `0.35`. | `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd xy_distance 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-11 (`support_type`, class a).** Given a per-region config overlay, when `support_type = "tree(auto)"` (canonical default `normal(auto)`), then `select_support_family` resolves the tree family and the producer's `effective_support_type` reports `is_tree()`, whereas the default resolves the traditional family. | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration support_family 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N1 (negative — bounds rejection).** Given the scheduler's manifest bounds layer, when `support_bottom_z_distance = -1.0` or `support_object_first_layer_gap = 11.0` (canonical max `10`), then resolution fails with the scheduler's out-of-bounds error naming the offending key rather than silently clamping. | `mkdir -p target && cargo test -p slicer-scheduler --test scheduler_integration config_bounds_enforcement 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`
- **AC-N2 (negative — returned keys must not be re-stubbed).** Given the two keys this packet returns to the queue, then neither planner manifest gains a table for them: `raft_first_layer_expansion` and `support_style` are absent from `traditional-support-planner.toml`, and the only two net-new `[config.schema.*]` tables across both planner manifests are `support_bottom_z_distance` and `support_object_first_layer_gap`. | `grep -q 'config.schema.raft_first_layer_expansion' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && echo FAIL || (grep -q 'config.schema.support_style' modules/core-modules/traditional-support-planner/traditional-support-planner.toml && echo FAIL || echo PASS)`
- **AC-N3 (negative — no padding edits).** Given Authoring rule 2, then this packet's diff contains no change to `ORCA_CONFIG_PADDING` in `crates/slicer-gcode/src/serialize.rs`. | `git diff --stat -- crates/slicer-gcode/src/serialize.rs | grep -q . && echo FAIL || echo PASS`
- **AC-N4 (regression guard — default path unchanged).** Given every one of the five newly built decision points left at its canonical default, then the support-plan output for the reference model is unchanged from the pre-packet baseline. This is a guard, never any key's evidence. | `mkdir -p target && cargo test -p traditional-support-planner --test traditional_family_tdd 2>&1 | tee target/test-output.log; grep -q "test result: ok" target/test-output.log && echo PASS || echo FAIL`

## Gate Commands

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo xtask check-literals`

## Docs Impact

- `docs/15_config_keys_reference.md` — regenerate; five keys move from declared-unread to live.
- `docs/specs/support-parity-gap-register.md` — row **G-05** (`support_bottom_z_distance` unimplemented) is closed by this packet. Report the destination-packet correction in the closing summary; do not edit the register from inside this packet.
- No ADR is required: no new claim, no new seam, no schema change.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportMaterial.cpp` — `detect_overhangs` (the `enforce_support_layers` forced branch and the `support_expansion` XY growth) and `PrintObjectSupportMaterial::top_contact_layers` (the overhang-cluster erode-and-measure smallness test borrowed by AC-3).
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::detect_overhangs` (the `support_critical_regions_only` clear-and-keep-cantilevers branch borrowed by AC-2) and `TreeSupport::draw_circles` (the `support_object_first_layer_gap` layer-0 substitution borrowed by AC-5 and AC-6).
- `OrcaSlicerDocumented/src/libslic3r/Slicing.cpp` — `SlicingParameters::create_from_config`, for `support_bottom_z_distance` becoming `gap_object_support`. The deliberately **not** borrowed part is `GCode::collect_layers_to_print`'s zero-means-fall-back-to-top rule; see `design.md` DIV-1.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `SupportParameters::SupportParameters`, for `gap_xy` and `gap_xy_first_layer`, and for the `support_style` resolution this packet deliberately does not port.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
