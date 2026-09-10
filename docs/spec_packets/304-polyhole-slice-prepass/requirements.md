# Requirements: 304-polyhole-slice-prepass

## Packet Metadata

- Grouped task IDs: none — this packet is queued by the wayfinder map, not by `docs/07_implementation_status.md`. Its backlog identity is wayfinder ticket 94 (P87).
- Backlog source: `docs/specs/orca-feature-gap/issues/94-author-packet-p87-quality-precision-new-polyhole.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

OrcaSlicer converts near-circular holes into regular polygons whose *inscribed* circle matches the requested radius, because an FDM printer's extrusion width makes a faceted circle print undersized. Canonical does this in `PrintObject::_transform_hole_to_polyholes` (`PrintObject.cpp`), a pass ported into OrcaSlicer from SuperSlicer, driven by four config keys. This tree implements none of it: `hole_to_polyhole`, `hole_to_polyhole_threshold` and `hole_to_polyhole_twisted` have **zero occurrences** in `crates/`, `modules/` or `xtask/` — not a manifest row, not a `ResolvedConfig` field, not even an `ORCA_CONFIG_PADDING` twin. A user setting them today gets silence.

The slice is coherent because all four keys are read by the same canonical function, in the same pass, over the same geometry, and none of them means anything without the others: the bool gates the pass, the threshold decides what counts as circular, and the twist and edge-cap parameterise the polygon that replaces the hole.

The one design decision this packet had to make is where the pass lives, and the answer overturns the tier table. `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` assigns all three queue keys to a "new polyhole module" (Tier C). That owner cannot work: canonical's grouping step accepts a hole only if a matching hole exists on a contiguous neighbouring layer, and its replacement step indexes the rotation set by the **absolute** layer index. Both are cross-layer reads. This tree's per-layer module stages are dispatched one layer at a time inside a layer-major executor (`execute_per_layer*` iterates layers in the outer loop and `plan.per_layer_stages` in the inner, `crates/slicer-runtime/src/layer_executor.rs`), so a module on `Layer::SlicePostProcess` can never see the neighbour it needs, and no barrier exists between per-layer stages where a host pass could stand in. The owner is therefore corrected to a host prepass built-in, the same reasoning packet `297-conical-overhang-slice-prepass` used to reject the module seam for its own cross-layer pass.

No packet is superseded or reopened.

## In Scope

- **Ungated pure kernel** `crates/slicer-core/src/algos/polyhole.rs`, opening with the standard porting header from `docs/ORCASLICER_ATTRIBUTION.md` (original C++ source path `src/libslic3r/PrintObject.cpp`), declared as `pub mod polyhole;` in `crates/slicer-core/src/algos/mod.rs` beside the ungated `bridge_over_infill`. Ungated deliberately: the kernel needs neither `rayon` nor `boostvoronoi`, and an ungated module cannot fall into the CLAUDE.md silent-zero-tests trap.
  - `pub fn create_polyholes(center: Point2, radius: i64, nozzle_diameter: i64, twisted: bool, max_edges: u32) -> Vec<Polygon>` — canonical's free function, including its edge-count formula, its circumscribed-radius correction, its five-rotation twist set, its output interleave, and clockwise winding.
  - `pub struct PolyholeParams { enabled: bool, threshold: ResolvedFloatOrPercent, twisted: bool, max_edges: u32, nozzle_diameter_units: i64, filament_id: u32 }` — the per-region resolution of the four keys plus the two canonical inputs that are not config (`nozzle_diameter` selected by the region's outer-wall filament, and that filament id, which participates in canonical's hole identity).
  - `pub fn transform_holes_to_polyholes(slices: &mut [SliceIR], region_params: &[Vec<Option<PolyholeParams>>]) -> usize` — the whole pass over already-committed slices, returning the number of holes replaced. `region_params` is indexed `[slice_index][region_index]`; `None` means that region is not enabled, which keeps every config decision in the producer and the kernel free of config types beyond `ResolvedFloatOrPercent`.
- **Detection, ported exactly**: candidacy (convex hole with more than eight points), centroid, min/max/mean vertex radius, min/max line-midpoint radius, `max_variation` from the threshold resolved against the **mean vertex radius** when it is a percentage, and both spread tests against `2 * max_variation`.
- **Grouping, ported exactly**: per-region timelines, an upward walk that stops at the first Z gap wider than the layer's own height, matching on filament id, centre distance and radius difference (both against `max_variation`), consuming matched holes, and keeping a group only when it spans at least two layers or is a lone hole on layer 0.
- **Replacement, ported exactly**: one `create_polyholes` set per group, indexed by `layer_index % set_len`, replacing the hole's point vector in place.
- **New host-only stage** `PrePass::PolyholeTransform`: one entry in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`) between `PrePass::Slice` and `PrePass::OverhangAnnotation`; one entry in `HOST_ONLY_STAGES` (`crates/slicer-scheduler/tests/contract/stage_list_consistency_tdd.rs`); one arm in `required_slots` (`crates/slicer-runtime/src/prepass.rs`) listing `LayerPlan`, `RegionMap` and `SliceIR`. `slicer_schema::VALID_STAGES` and `slicer_schema::STAGES` are deliberately **not** touched, which is what keeps the WIT surface, the `slicer-macros` glue match, the `pnp_cli module new` scaffold arm and `stage_io.rs`'s commit match out of scope.
- **Producer** `crates/slicer-runtime/src/builtins/polyhole_producer.rs` — `pub fn commit_polyhole_builtin(blackboard: &mut Blackboard) -> Result<(), PolyholeBuiltinError>`, declared in `crates/slicer-runtime/src/builtins/mod.rs`, registered by one `run_builtin_stage` call in `prepass.rs` with `should_run = |bb| bb.slice_ir().is_some() && bb.region_map().is_some()`, resolving per-region params through `RegionMapIR::config_for` and writing back through `Blackboard::replace_slice_ir`. One new `PrepassExecutionError::Polyhole` variant.
- **Config plumbing** in `crates/slicer-ir/src/resolved_config.rs`: four `cli` rows plus their `to_config_map` emissions, at canonical defaults — `hole_to_polyhole: bool = false`, `hole_to_polyhole_threshold: ResolvedFloatOrPercent = { value: 0.01, is_percent: false }`, `hole_to_polyhole_twisted: bool = true`, `hole_to_polyhole_max_edges: u32 = 50`.
- **Tests**: `crates/slicer-core/tests/algo_polyhole_tdd.rs` (auto-discovered, no `Cargo.toml` entry, no `required-features`, no file-level `cfg`), `crates/slicer-ir/tests/resolved_config_polyhole_tdd.rs`, and `crates/slicer-runtime/tests/executor/prepass_polyhole_stage_order_tdd.rs` plus its one `mod` line in `crates/slicer-runtime/tests/executor/main.rs`.
- **Docs and ledger**: the stage list in `docs/04_host_scheduler.md` and `docs/01_system_architecture.md` (numbered list and Data Dependency Matrix); one `docs/DEVIATION_LOG.md` row with the ID re-derived at write time; the P87 rows in `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (owner corrected) and `05-asset-packet-list.md` (P87 linked to this packet).

### Key disposition table

Declaration-only keys: 0

| Key | Canonical type / default | Owner after this packet | Decision point it drives | Disposition |
| --- | --- | --- | --- | --- |
| `hole_to_polyhole` | `coBool`, default `false` | `host:polyhole` built-in | per-region gate on the whole pass | wired — AC-3 / AC-4 / AC-11 |
| `hole_to_polyhole_threshold` | `coFloatOrPercent`, default `0.01` mm | `host:polyhole` built-in | `max_variation`, which decides whether a hole is circular enough to convert | wired — AC-5 |
| `hole_to_polyhole_twisted` | `coBool`, default `true` | `host:polyhole` built-in | one rotation reused on every layer vs. five rotations cycled by layer index | wired — AC-2 / AC-6 |
| `hole_to_polyhole_max_edges` | `coInt`, default `50` | `host:polyhole` built-in | upper cap on the polygon's edge count | wired — AC-7 (supporting non-queue key; see below) |

`hole_to_polyhole_max_edges` is **not** a queue key: it is absent from `docs/ORCA_CONFIG_REFERENCE.md`, which wayfinder tickets 04 and 05 already recorded as an inventory gap ("a 4th polyhole key exists in canonical but is missing from the 414 inventory ... not added to the queue here"). It is carried here because the alternative is a hardcoded `50` inside the kernel, which map rule 4 forbids. **Queue count unchanged: 409.** The same disposition packet 303 used for its five re-declared non-queue keys.

## Out of Scope

- **The elephant-foot ordering question.** Canonical runs elephant-foot before polyhole; packet 303 places elephant-foot on `Layer::SlicePostProcess`, after every prepass. This packet records the resulting inversion as a deviation clause and does not move, amend or depend on packet 303. If that question later resolves toward a prepass elephant-foot, this packet's only change is the position of one `run_builtin_stage` call.
- **Making the stage module-targetable.** No `VALID_STAGES` entry, no `slicer_schema::STAGES` row, no WIT world, no SDK trait, no `pnp_cli module new` scaffold arm. A fork wanting to replace this pass changes the built-in, not a manifest.
- **Visual-debug taps for the new stage.** `SILHOUETTE_TAP_STAGE_IDS` (`crates/pnp-cli/src/visual_debug.rs`) and the `BLACKBOARD_TAP_STAGE_IDS` list (`crates/slicer-runtime/src/layer_executor.rs`) are not extended; the pass's effect is observable on the existing post-prepass `SliceIR` taps.
- **Any `crates/slicer-gcode/src/serialize.rs` edit.** `ORCA_CONFIG_PADDING` is map rule 2 non-evidence; CONFIG_BLOCK emission for these keys rides as a side effect of the `ResolvedConfig` fields being live, and the bool word-form spelling defect stays with ticket 132.
- **Range validation.** Canonical's `min` on `hole_to_polyhole_max_edges` and `max_literal` on the threshold are GUI hints (map Notes, ticket 113). The kernel's `max(3, ..)` floor is canonical arithmetic and is ported; no key is rejected for being out of range.
- **The per-tool axis.** `nozzle_diameter` is read from the region's own resolved config, which already composes the `tool_config:<idx>:` overlay (ticket 118). This packet builds no second per-tool mechanism and is not blocked on ticket 125.

## Authoritative Docs

- `docs/04_host_scheduler.md` - over 300 lines; delegate a SUMMARY of the "Fixed Stage Order" section, then edit that list.
- `docs/01_system_architecture.md` - over 300 lines; delegate a SUMMARY of the numbered stage list and the Data Dependency Matrix; edit only those two places.
- `docs/08_coordinate_system.md` - direct read; governs every canonical constant the kernel converts.
- `docs/ORCASLICER_ATTRIBUTION.md` - direct read of the "Standard Porting Header" block only.

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `PrintObject::_transform_hole_to_polyholes` (detection, per-region gate, vertical grouping, replacement) and the free function `create_polyholes` (edge count, circumscribed radius, twist interleave) are the whole ported behaviour.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `PrintObject::slice` fixes the pass's position: after `slice_volumes` (XY compensation, elephant-foot, conical overhang) and `fix_slicing_errors`, before `groupingVolumesForBrim`.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — the four `hole_to_polyhole*` declarations, for types and defaults only; their `min` and `max_literal` are GUI hints and are deliberately not borrowed.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-14`. Refinements not stated in their Given/When/Then text: AC-1's edge-count arithmetic uses canonical's `0.4` literal as a *reference nozzle diameter*, not this tree's configured nozzle — the formula is `4 * radius_mm * 0.4 / nozzle_diameter_mm`, so it degenerates to `4 * radius_mm` at a 0.4 mm nozzle. AC-3's "outer contour unchanged" is asserted on the point vector, not on area, because the pass must never touch a contour. AC-12's command deliberately asserts a **non-zero** test count rather than a green exit, since a feature-gated file would print `ok` with zero tests.
- Negative: `AC-N1` through `AC-N4`.
- Cross-packet impact:
  - **Packet 297 (`297-conical-overhang-slice-prepass`, draft).** Canonical calls `apply_conical_overhang` at the end of `slice_volumes`, before `_transform_hole_to_polyholes`. If 297 lands, its built-in must be registered **before** this one in `prepass.rs`. Whichever packet lands second owns checking the order; this packet records the requirement.
  - **Packet 303 (`303-elefant-foot-slice-postprocess`, draft).** 303 puts elephant-foot on `Layer::SlicePostProcess`, which runs after every prepass stage, so in this port polyhole runs **before** elephant-foot where canonical runs it after. See the deviation clause in `design.md`; the packet records it and changes nothing in 303.
  - **Wayfinder ticket 123** (gap-source completeness audit) gains a second confirmed instance of a live canonical key missing from `docs/ORCA_CONFIG_REFERENCE.md`: `hole_to_polyhole_max_edges`, already flagged by tickets 04 and 05 and now made live by this packet.
  - **Wayfinder assets.** `04-asset-tier-assignment.md`'s three P87 rows change owner from `new polyhole module` to the host built-in; `05-asset-packet-list.md`'s P87 heading gains this packet's number and drops "new module".

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p slicer-core --test algo_polyhole_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Kernel: AC-1 through AC-7, AC-N1 through AC-N4 | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo test -p slicer-ir --test resolved_config_polyhole_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Config plumbing and canonical defaults: AC-8 | FACT pass/fail |
| `cargo test -p slicer-scheduler --test contract stage_list_consistency_tdd 2>&1 \| tee target/test-output.log \| rg 'test result'` | Stage-list partition holds with the new host-only stage: AC-9 | FACT pass/fail |
| `cargo test -p slicer-runtime --test executor prepass_polyhole_stage_order_tdd 2>&1 \| tee target/test-output.log \| rg 'test result: ok\. [1-9]'` | Built-in registration, ordering and blackboard write-back: AC-10, AC-11. Asserts a **non-zero** test count: an unregistered file in the aggregated `executor` binary reports 0 tests and reads as green. | FACT pass/fail |
| `cargo check --workspace --all-targets` | Whole-tree compile including test targets | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Lint gate, required before committing | FACT pass/fail |
| `cargo xtask check-literals` | Struct-literal churn gate, required before committing | FACT pass/fail |
| `cargo xtask check-deviations` | The new `DEV-###` row parses and is unique | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

- **The kernel lands before the producer.** Steps 1 and 2 leave `slicer-core` compiling and fully tested with nothing registered in the scheduler; the producer in Steps 4 and 5 only ever calls an already-proven kernel. Do not interleave.
- **Ungatedness is a cross-step invariant.** Any step touching `crates/slicer-core/src/algos/mod.rs` or `crates/slicer-core/Cargo.toml` must leave `polyhole` free of `cfg(feature = "host-algos")` and free of `required-features`. Re-check after every edit to either file; a regression here is silent — a clean-looking green run with zero tests compiled.
- **The stage entry and its partition entry land in the same step.** Adding `PrePass::PolyholeTransform` to `STAGE_ORDER` without adding it to `HOST_ONLY_STAGES` breaks `host_only_stages_partition_stage_order_into_valid_stages`. Step 3 owns both edits and runs the contract test before exiting.
- **No guest WASM is touched.** Nothing in this packet's change surface feeds a guest build: the kernel is host-only in practice (no module links it), the producer lives in `slicer-runtime`, and no manifest changes. `cargo xtask build-guests --check` is therefore not a per-step gate here — but if a step ever adds a `modules/` path, that immediately stops being true.

## Context Discipline Notes

- `crates/slicer-runtime/src/slice_postprocess_prepass.rs` is the shape to copy but is a long file: read only `commit_shell_classification_builtin` and the three helpers `build_region_timelines`, `find_region_mut`, `clone_region_polys`. Do not read its shell-classification passes.
- `crates/slicer-ir/src/resolved_config.rs` is ~3000 lines and mostly macro machinery. Locate an existing `cli` row of each needed type by grep (`extract_bool`, `extract_float_or_percent`, `extract_int_as_u32`) and copy its shape; never read the file top to bottom.
- `crates/slicer-runtime/src/prepass.rs` is long. Read only the `run_builtin_stage` definition, the `PrepassExecutionError` enum, the contiguous block of `run_builtin_stage` calls around `PrePass::Slice` and `PrePass::OverhangAnnotation`, and `required_slots`.
- Every `OrcaSlicerDocumented/` fact in this packet is already stated; re-dispatch only to confirm a specific formula before porting it, never to re-derive the pass.
