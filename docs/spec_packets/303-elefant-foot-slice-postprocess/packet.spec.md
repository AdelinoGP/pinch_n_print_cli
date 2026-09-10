---
status: draft
packet: 303-elefant-foot-slice-postprocess
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/93-author-packet-p86-quality-precision-new-elefant-foot.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 93 (P86). First Tier-C new-module packet in the queue.
---

# Packet Contract: 303-elefant-foot-slice-postprocess

## Goal

Make the P86 key pair drive a real elephant-foot compensation at parity with canonical `elephant_foot_compensation` (`ElephantFootCompensation.cpp`) and its `PrintObject::slice_volumes` call site — a width-limited variable inward offset of the first `elefant_foot_compensation_layers` layers, linearly tapered by layer index and gated off when a raft is present — delivered as the first production module on the existing, currently-unoccupied `Layer::SlicePostProcess` stage.

## Scope Boundaries

P86 is two Tier-C keys whose only occurrence in this tree today is the `ORCA_CONFIG_PADDING` spelling twin for `elefant_foot_compensation` in `crates/slicer-gcode/src/serialize.rs` — rule 2, not evidence. Claim-time grounding (ticket 93) holds both in: each is live in canonical's slicing pipeline (`PrintObject::slice_volumes` in `PrintObjectSlice.cpp` computes the per-layer taper; `Brim.cpp::use_brim_efc_outline` and `Fill.cpp` read them downstream; neither is dead-in-canonical) and neither has any decision point here. The tier table's `new elefant-foot module` owner is **confirmed** and sharpened to the seam: a new core module `modules/core-modules/elefant-foot/` declaring `[stage] id = "Layer::SlicePostProcess"`, whose pure kernel lands ungated in `crates/slicer-core/src/algos/` beside `bridge_over_infill` (so it compiles for `wasm32` and links into the guest, the `gyroid-infill` dependency precedent). The module replaces region footprints through the existing `slice-postprocess-builder`'s `set-polygons` — no WIT change, no IR field, no new host service, no new claim. Rule 4 does not fire: elephant-foot compensation is one parameterised geometric correction with no canonical alternative implementation, so there is no claim holder to create (`[claims] holds = []`, `requires = []`). Canonical's `min` bounds (`0` and `1`) are borrowed; the GUI's `> 1` mm clamp in `ConfigManipulation.cpp` is a GUI hint under the ticket-113 rule, so the packet performs no range rejection beyond the manifest's declared `min`. CONFIG_BLOCK emission rides as a live-key side effect only; the padding twin is untouched and its spelling stays with ticket 132.

## Prerequisites and Blockers

- Depends on: nothing. Every symbol below is live on HEAD (verified at authoring): the `Layer::SlicePostProcess` entry in `STAGE_ORDER` (`crates/slicer-scheduler/src/execution_plan.rs`), the `slice-postprocess-builder` resource with `set-polygons` (`crates/slicer-schema/wit/deps/ir-types.wit`), the `SlicePostprocessBuilder::set_polygons` SDK binding (`crates/slicer-sdk/src/builders.rs`), the `slice-region-view` accessors `polygons` / `object-id` / `region-id` / `effective-layer-height` / `variant-chain`, `slicer_core::flow::{resolve_role_width, line_width_to_spacing, RoleWidthContext}`, `slicer_core::polygon_ops::{union, offset, difference}`, and `pnp_cli module new`'s `Layer::SlicePostProcess` scaffold arm (`crates/pnp-cli/src/module_new.rs`).
- Related work, not a blocker: ticket 95 / P88 (`xy_contour_compensation`, `xy_hole_compensation`) shares canonical's call site and must compose on this same stage — see the ordering obligation in `design.md`; ticket 132 (CONFIG_BLOCK reader contract — the padding spelling rides there); ticket 128 (`float_or_percent` units mismatch — this packet reads `outer_wall_line_width` through the config view's `get_abs_value` resolver and inherits whatever that ticket rules); ticket 125 (per-tool model — explicitly NOT this packet's axis: both keys are canonical `PrintObjectConfig` per-object keys carried by the existing per-object overlay).
- Unblocks: wayfinder ticket 93 (P86 closes when this packet is authored). `brim_use_efc_outline` (P05, `shed-to-queue` per `docs/specs/orca-feature-gap/issues/key-correction-inventory.md`) needs EFC geometry to exist before it can mean anything; this packet supplies the geometry but deliberately does **not** implement that key — see §Out of Scope in `requirements.md`.
- Activation blockers: none. One new deviation row is minted; re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` plus `docs/spec_packets/*/` at write time rather than trusting any ID written here (`DEV-198` was next-free at authoring).

## Acceptance Criteria

State ACs only here; `requirements.md` references their IDs.

- **AC-1. Given** a 20 mm x 20 mm square `ExPolygon` and `compensation = 0.0`, **when** `slicer_core::algos::elephant_foot::elephant_foot_compensation` is called with any `min_contour_width`, **then** the returned `ExPolygon` equals the input — same contour point count, every point identical. | `cargo test -p slicer-core --test algo_elephant_foot_tdd zero_compensation_is_identity 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** a 20 mm x 20 mm square `ExPolygon`, `min_contour_width = 0.757` mm and `compensation = 0.2` mm, **when** the kernel runs, **then** the output area is strictly less than the input area and the output lies strictly inside the input (`slicer_core::polygon_ops::difference(output, input)` is empty). | `cargo test -p slicer-core --test algo_elephant_foot_tdd square_shrinks_and_stays_inside 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** an `ExPolygon` whose bounding-box X or Y extent is below `min_contour_width + 2 * compensation`, or whose area is below `5 * (min_contour_width + 2 * compensation)^2`, **when** the kernel runs, **then** the input is returned unchanged — canonical's "the contour is tiny, don't correct it" early-out. | `cargo test -p slicer-core --test algo_elephant_foot_tdd tiny_contour_is_returned_unchanged 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** a 20 mm square with a 2 mm-wide rib attached, **when** the kernel runs with `min_contour_width = 0.757` mm and `compensation = 0.2` mm, **then** the rib's local width is reduced by strictly less than `2 * compensation` while the square body's local width is reduced by `2 * compensation` to within `0.02` mm — the width-limited behaviour that distinguishes this kernel from a uniform `offset(-compensation)`. | `cargo test -p slicer-core --test algo_elephant_foot_tdd narrow_feature_shrinks_less_than_uniform_offset 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `elefant_foot_compensation = 0.0` (the canonical default) and any layer index, **when** the `elefant-foot` module's `run_slice_postprocess` executes, **then** it records zero polygon updates on the `SlicePostprocessBuilder` (`polygon_updates()` is empty), so a default slice is byte-identical to one taken with the module absent. | `cargo test -p elefant-foot --test elefant_foot_tdd default_compensation_writes_no_updates 2>&1 | tee target/test-output.log | tail -5`
- **AC-6. Given** `elefant_foot_compensation = 0.2` and `elefant_foot_compensation_layers = 3`, **when** the module runs at layer indices 0, 1, 2 and 3, **then** the compensation applied is `0.2`, `0.2 - 0.2/3`, `0.2 - 2*(0.2/3)` and none at all (zero updates at index 3) — canonical `slice_volumes`' `elfoot = efc - (efc / layers) * layer_id` under its `layer_id < layers` guard. | `cargo test -p elefant-foot --test elefant_foot_tdd taper_matches_canonical_per_layer_formula 2>&1 | tee target/test-output.log | tail -5`
- **AC-7. Given** `elefant_foot_compensation = 0.2` and `support_raft_layers = 3`, **when** the module runs at layer index 0, **then** it records zero polygon updates — canonical enables EFC only when `raft_layers == 0` ("Only enable Elephant foot compensation if printing directly on the print bed"). | `cargo test -p elefant-foot --test elefant_foot_tdd raft_present_disables_compensation 2>&1 | tee target/test-output.log | tail -5`
- **AC-8. Given** a layer carrying two regions with different `region_id`s and a non-empty `variant_chain`, **when** the module writes updates, **then** it emits one `set_polygons` call per region whose `RegionKey` carries that region's own `object_id`, `region_id` and `variant_chain` plus the dispatched `layer_index` — never an empty `variant_chain`. | `cargo test -p elefant-foot --test elefant_foot_tdd region_key_carries_variant_chain 2>&1 | tee target/test-output.log | tail -5`
- **AC-9. Given** the manifest `modules/core-modules/elefant-foot/elefant-foot.toml`, **when** it is parsed, **then** `[stage] id` is `"Layer::SlicePostProcess"`, `[claims] holds` and `requires` are both empty, `[ir-access] reads` and `writes` are `["SliceIR"]`, and `[config.schema]` declares `elefant_foot_compensation` (`type = "float"`, `default = 0.0`, `min = 0.0`) and `elefant_foot_compensation_layers` (`type = "int"`, `default = 1`, `min = 1`) at canonical defaults. | `cargo test -p elefant-foot --test slicer_module_binding_tdd manifest_declares_stage_claims_and_canonical_defaults 2>&1 | tee target/test-output.log | tail -5`
- **AC-10. Given** the built `elefant-foot` component, **when** the guest freshness gate runs, **then** it exits `0` and `modules/core-modules/elefant-foot/elefant-foot.wasm` exists with its embedded WIT world resolving to `slicer:layer-slice-postprocess/slice-postprocess-module`. | `cargo xtask build-guests --check; echo "exit=$?"`
- **AC-11. Given** the packet's disposition table, **when** it is read, **then** it lists exactly the 2 P86 keys, both wired to the compensation pass, with zero declaration-only keys. | `rg -q 'elefant_foot_compensation_layers.*wired' docs/spec_packets/303-elefant-foot-slice-postprocess/requirements.md && rg -q 'declaration-only keys: 0' docs/spec_packets/303-elefant-foot-slice-postprocess/requirements.md; echo "exit=$?"`

## Negative Test Cases

- **AC-N1. Given** `elefant_foot_compensation = 0.2` and a resolved outer-wall width and layer height whose `slicer_core::flow::line_width_to_spacing` result is non-positive, **when** the module runs, **then** it returns a fatal `ModuleError` naming the negative-spacing condition rather than silently substituting a width — the `NegativeSpacingError` contract in `slicer_core::flow`. | `cargo test -p elefant-foot --test elefant_foot_tdd negative_spacing_is_fatal_module_error 2>&1 | tee target/test-output.log | tail -5`
- **AC-N2. Given** `elefant_foot_compensation_layers = 0` in the raw config, **when** module config is validated against the manifest, **then** the value is rejected by the declared `min = 1` bound and the module never runs with it. | `cargo test -p elefant-foot --test slicer_module_binding_tdd layers_below_manifest_min_is_rejected 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets 2>&1 | tee target/test-output.log | tail -3`
- `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tee target/test-output.log | tail -3`
- `cargo test -p slicer-core --test algo_elephant_foot_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/01_system_architecture.md` - delegated SUMMARY (layer-stage ownership; the claim-system rule-4 trigger test)
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (module manifest TOML schema: `[stage]`, `[claims]`, `[ir-access]`, `[config.schema]` field rules)
- `docs/08_coordinate_system.md` - direct range on the mm-to-unit helpers (every offset and area boundary in the kernel)
- `docs/13_slicer_helpers_crate.md` - direct read (which polygon primitives already exist, before adding any)
- `docs/ORCASLICER_ATTRIBUTION.md` - direct read (the porting header is mandatory on the kernel file)
- `docs/21_data_defaults_and_fixtures.md` - direct read (the `check-literals` rule governs every `RoleWidthContext` literal in the new tests)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` - P86 rows (owner/tier confirmation)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` - P86 entry (membership)

## Doc Impact Statement (Required)

- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` section "Quality / Precision" - `rg -q 'elefant_foot_compensation.*303-elefant-foot' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (Step 7: owner confirmed `new elefant-foot module`, packet linkage)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` section "P86" - `rg -q '303-elefant-foot' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (Step 7: 2 keys in at packet 303)
- `docs/DEVIATION_LOG.md` new row - `rg -q 'elefant_foot_compensation' docs/DEVIATION_LOG.md` (Step 7: the seam and uncompensated-outline divergences; re-derive the next free `DEV-###` at write time)
- `docs/config/host-keys.toml` - **no edit**: both keys are module-manifest-owned, not `[host_runtime]` keys, so `cargo xtask gen-config-docs` regeneration is not triggered. Verified by `rg -q 'elefant_foot' docs/config/host-keys.toml; test $? -ne 0`

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/ElephantFootCompensation.cpp` — `elephant_foot_compensation` (both `ExPolygon` arities plus the `Flow` overload that derives `min_contour_width` as `width + spacing`): borrow the whole pass shape — the tiny-contour early-out, the `SCALED_EPSILON` simplify, the contour resample, the per-point distance and delta computation, `smooth_compensation_banded`, and the variable inward offset. This is the packet's primary borrow.
- `OrcaSlicerDocumented/src/libslic3r/PrintObjectSlice.cpp` — `PrintObject::slice_volumes`: borrow the `raft_layers == 0` gate, the `layer_id < elefant_foot_compensation_layers` guard, the `elfoot = efc - (efc / layers) * layer_id` taper, and the union of the kernel result. The `lslices_elfoot_uncompensated` store is a **named non-borrow** — see the deviation row.
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params`: borrow the two defaults exactly (`elefant_foot_compensation` coFloat `0.`, min `0`; `elefant_foot_compensation_layers` coInt `1`, min `1`).
- `OrcaSlicerDocumented/src/libslic3r/Brim.cpp` — `use_brim_efc_outline` (named non-borrow: `brim_use_efc_outline` is out of scope here, and this tree's brim is bbox-derived).
- `OrcaSlicerDocumented/src/libslic3r/Fill/Fill.cpp` — the `elefant_foot_layers_density` solid-infill density arm (named non-borrow: that key is absent from this repo's gap source entirely — surfaced to wayfinder ticket 123).
- `OrcaSlicerDocumented/src/slic3r/GUI/ConfigManipulation.cpp` — the `> 1` mm clamp (named non-borrow: GUI hint, not slicing validation, per the ticket-113 rule).

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
