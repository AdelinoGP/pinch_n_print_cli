---
status: draft
packet: 304-polyhole-slice-prepass
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/94-author-packet-p87-quality-precision-new-polyhole.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 94 (P87). Corrects the tier table's owner from "new polyhole module" to a host prepass built-in.
---

# Packet Contract: 304-polyhole-slice-prepass

## Goal

Make the P87 key set drive a real hole-to-polyhole conversion at parity with canonical `PrintObject::_transform_hole_to_polyholes` and the free function `create_polyholes` (`PrintObject.cpp`) — detect near-circular convex holes that persist across contiguous layers and replace each with a circumscribing regular polygon sized from the nozzle diameter, optionally rotated per layer — delivered as a new host prepass built-in `host:polyhole` on a new host-only stage `PrePass::PolyholeTransform` between `PrePass::Slice` and `PrePass::OverhangAnnotation`.

## Scope Boundaries

P87 is three Tier-C queue keys with **zero occurrences anywhere in this tree** — unlike most queue packets there is not even an `ORCA_CONFIG_PADDING` twin to disregard. A fourth canonical key, `hole_to_polyhole_max_edges`, is read by the same canonical function and is carried here as a **supporting non-queue key**: it is absent from `docs/ORCA_CONFIG_REFERENCE.md` and was already flagged as an inventory gap by wayfinder tickets 04 and 05, so declaring it leaves the 409-key queue count unchanged (the packet 303 precedent for re-declared non-queue keys). The tier table's owner — `new polyhole module`, Tier C — is **corrected** here under ticket 27's re-derive-the-owner rule: the pass is cross-layer (a hole qualifies only if it persists across at least two contiguous layers, and the twist index is the absolute layer index), and this tree's layer executor is layer-major (`execute_per_layer*` loops layers outer and `plan.per_layer_stages` inner, `crates/slicer-runtime/src/layer_executor.rs`), so no per-layer module stage and no barrier between per-layer stages can serve it. Rule 4 does not fire: the four keys are scalar parameters of one geometric pass, not an enum selecting between competing algorithms, so no claim is minted. Canonical's `min` on `hole_to_polyhole_max_edges` is a GUI hint under the ticket-113 rule; the kernel's own `max(3, ..)` floor is canonical *behaviour*, not a borrowed bound, and the packet adds no range rejection. CONFIG_BLOCK emission rides as a live-key side effect only; `crates/slicer-gcode/src/serialize.rs` is never opened and the bool word-form spelling defect stays with ticket 132.

## Prerequisites and Blockers

- Depends on: nothing. Every symbol below is live on HEAD (verified at authoring): `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`), `HOST_ONLY_STAGES` (`crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`), `run_builtin_stage` and `required_slots` (`crates/slicer-runtime/src/prepass.rs`), `Blackboard::replace_slice_ir` (`crates/slicer-runtime/src/blackboard.rs`), `commit_shell_classification_builtin` with its `build_region_timelines` / `find_region_mut` / `clone_region_polys` helpers (`crates/slicer-runtime/src/slice_postprocess_prepass.rs`), `RegionMapIR::config_for` and `RegionKey` (`crates/slicer-ir/src/slice_ir.rs`), `ExPolygon` / `Polygon` / `Point2::from_mm` (`crates/slicer-ir/src/slice_ir.rs`), `ResolvedFloatOrPercent` with `extract_float_or_percent` / `extract_bool` / `extract_int_as_u32` (`crates/slicer-ir/src/resolved_config.rs`), and the ungated `bridge_over_infill` precedent in `crates/slicer-core/src/algos/mod.rs`.
- Related work, not a blocker: packet `297-conical-overhang-slice-prepass` (draft — `crates/slicer-runtime/src/builtins/conical_overhang_producer.rs` does **not** exist yet) must register its built-in **before** this one to keep canonical's order; packet `303-elefant-foot-slice-postprocess` (draft) places elephant-foot on `Layer::SlicePostProcess`, which inverts canonical's elephant-foot-then-polyhole order — see the deviation clause in `design.md` §Data and Contract Notes; ticket 132 (CONFIG_BLOCK reader contract); ticket 128 (`float_or_percent` unit mismatch — **not applicable at this seam**, because the threshold is resolved host-side by `extract_float_or_percent` and never through a guest `ConfigView::get_abs_value`).
- Unblocks: wayfinder ticket 94 (P87 closes when this packet is authored).
- Activation blockers: none. One new deviation row is minted; re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` plus `docs/spec_packets/*/` at write time rather than trusting any ID written here (`DEV-198` was next-free at authoring, and packet 303 also intends to file one).

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** `center = (0,0)`, a radius of 5 mm in internal units, a nozzle diameter of 0.4 mm, `twisted = false` and `max_edges = 50`, **when** `slicer_core::algos::polyhole::create_polyholes` runs, **then** it returns exactly one `Polygon` whose point count is `min(50, max(3, round(4.0 * 5.0 * 0.4 / 0.4))) = 20`, with every point at distance `5.0 / cos(PI/20)` mm from the centre to within 1 internal unit, wound clockwise. | `cargo test -p slicer-core --test algo_polyhole_tdd create_polyholes_edge_count_and_circumscribed_radius 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** the AC-1 inputs with `twisted = true`, **when** `create_polyholes` runs, **then** it returns exactly 5 `Polygon`s, each with the AC-1 point count and circumscribed radius, laid out by canonical's interleave (the polygon built for iteration `i` is stored at index `i / 2` when `i` is even and at `(5 + 1) / 2 + i / 2` when `i` is odd), so consecutive layer indices alternate rather than progress monotonically in rotation. | `cargo test -p slicer-core --test algo_polyhole_tdd twisted_returns_five_interleaved_rotations 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** three contiguous `SliceIR` layers each carrying one region whose single `ExPolygon` has a 32-point circular hole of radius 3 mm at the same centre, and `PolyholeParams { enabled: true, .. }` for every region, **when** `slicer_core::algos::polyhole::transform_holes_to_polyholes` runs, **then** it returns `3`, each replaced hole's point count equals the `create_polyholes` edge count for radius 3 mm, and every layer's outer contour point vector is unchanged. | `cargo test -p slicer-core --test algo_polyhole_tdd contiguous_circular_hole_is_replaced_on_every_layer 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** the AC-3 fixture with `enabled: false` for every region (the canonical `hole_to_polyhole` default), **when** `transform_holes_to_polyholes` runs, **then** it returns `0` and every `SliceIR` compares `==` to its pre-call clone — a default slice is identical to one taken with the stage absent. | `cargo test -p slicer-core --test algo_polyhole_tdd disabled_is_bitwise_identity 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** the AC-3 fixture with the hole's vertex radii jittered so `diameter_max - diameter_min` is 0.05 mm, **when** the pass runs with `threshold = ResolvedFloatOrPercent { value: 0.01, is_percent: false }` (the canonical default), **then** it returns `0`; **and when** it runs with `ResolvedFloatOrPercent { value: 0.5, is_percent: false }`, **then** it returns `3`. | `cargo test -p slicer-core --test algo_polyhole_tdd threshold_gates_detection_at_non_default_value 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** the AC-3 fixture with `twisted: false`, **when** the pass runs, **then** the three replaced holes have identical point vectors; **and given** `twisted: true` (the canonical default), **then** the layer-0 and layer-1 holes differ and each equals `create_polyholes(..)[layer_index % 5]`. | `cargo test -p slicer-core --test algo_polyhole_tdd twisted_false_reuses_one_rotation 2>&1 | tee target/test-output.log | tail -5`
- **AC-7. Given** the AC-3 fixture carrying a hole of radius 12 mm (whose uncapped edge count is 48) and `max_edges = 6`, **when** the pass runs, **then** every replaced hole has exactly 6 points; **and with** `max_edges = 50` (the canonical default) the same hole yields 48. | `cargo test -p slicer-core --test algo_polyhole_tdd max_edges_caps_edge_count 2>&1 | tee target/test-output.log | tail -5`
- **AC-8. Given** a raw config source spelling `hole_to_polyhole = true`, `hole_to_polyhole_threshold = "5%"`, `hole_to_polyhole_twisted = false` and `hole_to_polyhole_max_edges = 6`, **when** a `ResolvedConfig` is resolved from it and round-tripped through `to_config_map`, **then** the four fields hold `true`, `ResolvedFloatOrPercent { value: 5.0, is_percent: true }`, `false` and `6`; **and** an empty raw source yields the canonical defaults `false`, `ResolvedFloatOrPercent { value: 0.01, is_percent: false }`, `true` and `50`. | `cargo test -p slicer-ir --test resolved_config_polyhole_tdd 2>&1 | tee target/test-output.log | tail -5`
- **AC-9. Given** `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`), **when** the stage-list contract test runs, **then** `"PrePass::PolyholeTransform"` appears exactly once, strictly after `"PrePass::Slice"` and strictly before `"PrePass::OverhangAnnotation"`, is listed in `HOST_ONLY_STAGES`, and is absent from `slicer_schema::VALID_STAGES` so no module manifest may target it. | `cargo test -p slicer-scheduler --test contract stage_list_consistency_tdd 2>&1 | tee target/test-output.log | rg 'test result'`
- **AC-10. Given** a blackboard carrying a committed `SliceIR` and `RegionMapIR` whose region `ResolvedConfig` has `hole_to_polyhole = true`, **when** `run_prepass` executes, **then** the `PrePass::PolyholeTransform` built-in runs after `PrePass::Slice` and before `PrePass::OverhangAnnotation`, and the post-prepass `SliceIR` read back off the blackboard holds the faceted hole rather than the circular one. The test function is named `polyhole_runs_between_slice_and_overhang_annotation` in `crates/slicer-runtime/tests/executor/prepass_polyhole_stage_order_tdd.rs`, registered with a `mod` line in `crates/slicer-runtime/tests/executor/main.rs` — without that registration the file never compiles and the filter reports a false green on zero tests. | `cargo test -p slicer-runtime --test executor polyhole_runs_between_slice_and_overhang_annotation 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"`
- **AC-11. Given** two regions on one layer whose interned `ResolvedConfig`s differ (`hole_to_polyhole = true` for one, `false` for the other), **when** the built-in runs, **then** only the enabled region's hole is replaced — the producer resolves the gate through `RegionMapIR::config_for(&RegionKey { global_layer_index, object_id, region_id, variant_chain })`, matching canonical's per-region `region().config()` read, never a single global config. The test function is named `per_region_gate_converts_only_the_enabled_region`, in the same file and behind the same `mod` registration as AC-10. | `cargo test -p slicer-runtime --test executor per_region_gate_converts_only_the_enabled_region 2>&1 | tee target/test-output.log | rg 'test result: ok\. [1-9]'; echo "exit=$?"`
- **AC-12. Given** `crates/slicer-core/src/algos/mod.rs` and `crates/slicer-core/Cargo.toml`, **when** a **bare** `cargo test -p slicer-core` (no `--features`) runs the kernel's test binary, **then** it reports a **non-zero** passing test count, and `crates/slicer-core/Cargo.toml` declares no `algo_polyhole_tdd` target — proving the module and its test binary are ungated, so the CLAUDE.md silent-zero-tests trap (a gated file printing `ok` with zero tests) cannot fire. | `rg -q '^pub mod polyhole;' crates/slicer-core/src/algos/mod.rs && ! rg -q 'algo_polyhole_tdd' crates/slicer-core/Cargo.toml && cargo test -p slicer-core --test algo_polyhole_tdd 2>&1 | tee target/test-output.log | rg -q 'test result: ok\. [1-9]'; echo "exit=$?"`
- **AC-13. Given** the packet's disposition table, **when** it is read, **then** it lists the three P87 queue keys plus `hole_to_polyhole_max_edges`, every one wired to the detection or construction decision point, with zero declaration-only keys and an explicit statement that the queue count is unchanged. | `rg -qi 'declaration-only keys: 0' docs/spec_packets/304-polyhole-slice-prepass/requirements.md && rg -qi 'queue count unchanged: 409' docs/spec_packets/304-polyhole-slice-prepass/requirements.md; echo "exit=$?"`
- **AC-14. Given** the docs edited by this packet, **when** they are read, **then** the new stage appears in the fixed stage order and in the architecture stage list, and the deviation is discoverable from the config-key reference. | `rg -q 'PrePass::PolyholeTransform' docs/04_host_scheduler.md && rg -q 'PrePass::PolyholeTransform' docs/01_system_architecture.md && rg -q 'hole_to_polyhole' docs/15_config_keys_reference.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** a hole with eight or fewer points, or a hole carrying at least one reflex vertex (an L-shaped hole), **when** the pass runs with `enabled: true` and a generous threshold, **then** it returns `0` — canonical's convexity-and-point-count guard. | `cargo test -p slicer-core --test algo_polyhole_tdd non_convex_or_low_point_holes_are_rejected 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** a square hole resampled to 32 points (convex, above the point-count floor, small vertex-radius spread but large line-midpoint-radius spread), **when** the pass runs, **then** it returns `0` — rejected by the line-midpoint test that stops canonical turning square holes into polyholes. | `cargo test -p slicer-core --test algo_polyhole_tdd square_hole_is_rejected_by_line_midpoint_test 2>&1 | tee target/test-output.log | tail -5`
- **AC-N3. Given** a qualifying circular hole present on exactly one layer, **when** that layer's index is 3, **then** it is not replaced; **when** that layer's index is 0, **then** it is replaced — canonical's at-least-two-layers rule and its lone-first-layer rescue. | `cargo test -p slicer-core --test algo_polyhole_tdd single_layer_hole_replaced_only_on_layer_zero 2>&1 | tee target/test-output.log | tail -5`
- **AC-N4. Given** two qualifying holes at the same centre and radius on layers 0 and 2 with layer 1 absent from that region's timeline (a Z gap wider than the layer height), **when** the pass runs, **then** neither is replaced — canonical breaks the upward walk on the contiguity test, so a non-contiguous pair never forms a group. | `cargo test -p slicer-core --test algo_polyhole_tdd non_contiguous_layers_do_not_group 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p slicer-core --test algo_polyhole_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/04_host_scheduler.md` - over 300 lines; delegate a SUMMARY of the "Fixed Stage Order" section only, then edit that one list.
- `docs/01_system_architecture.md` - over 300 lines; delegate a SUMMARY of the numbered stage list and the Data Dependency Matrix; edit only those two places.
- `docs/08_coordinate_system.md` - direct read; the 1 unit = 100 nm porting checklist that governs every canonical constant in the kernel.
- `docs/ORCASLICER_ATTRIBUTION.md` - direct read of the "Standard Porting Header" block; the kernel file must open with it verbatim.

## Doc Impact Statement (Required)

- `docs/04_host_scheduler.md` section "Fixed Stage Order" - `rg -q 'PrePass::PolyholeTransform' docs/04_host_scheduler.md`
- `docs/01_system_architecture.md` sections "stage list" and "Data Dependency Matrix" - `rg -q 'PrePass::PolyholeTransform' docs/01_system_architecture.md`
- `docs/15_config_keys_reference.md` section "Deviations from OrcaSlicer" - `rg -q 'hole_to_polyhole' docs/15_config_keys_reference.md`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::_transform_hole_to_polyholes` (detection, per-region gate, vertical grouping, replacement) and the free function `create_polyholes` (edge count, circumscribed radius, twist interleave) are the whole ported behaviour.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `PrintObject::slice` fixes the pass's position: after `slice_volumes` (XY compensation, elephant-foot, conical overhang) and `fix_slicing_errors`, before `groupingVolumesForBrim`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the four `hole_to_polyhole*` declarations, for types and defaults only; their `min` and `max_literal` are GUI hints and are deliberately not borrowed.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
