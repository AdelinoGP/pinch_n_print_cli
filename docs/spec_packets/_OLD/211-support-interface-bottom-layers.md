---
status: superseded
packet: 211-support-interface-bottom-layers
task_ids:
  - TASK-327
superseded_by: 210a-support-planner-coord-t + 210b-support-interface-bottom-layers
---

# 211-support-interface-bottom-layers

## Goal

Replace `support_interface_bottom_layers`' warn-only code-1003 stub in `support-planner` with real bottom-interface (floor) bands: resolve the canonical `< 0 ⇒ use the top count` fallback, detect where each smoothed branch chain lands on model geometry below, and densify the lowest N layers of every model-landing chain with the same scan-line fill the top-interface band already uses.

## Problem Statement

`DEV-129` (`docs/DEVIATION_LOG.md`) records that `support_interface_bottom_layers` is registered, parsed, and inert. `support-planner.toml` declares it under a `# Not yet implemented` comment with `default = -1`, and `SupportPlanner::run_support_geometry` reads it only to `push_diagnostic` a code-1003 `Warn` — `"…is not yet implemented (config value={interface_bottom_layers})"` — before the layer loop. No geometry is produced, so PnP's support interfaces are top-only. Canonical implements it: `number_of_support_interface_bottom_layers` (`SupportParameters.hpp`) applies a `< 0 ⇒ use the top count` fallback, `TreeSupportCommon.hpp` turns the result into `support_floor_enable` / `support_floor_layers`, and `TreeSupport::draw_circles` builds real `floor_areas` from it.

The warn-only state is deliberate and test-pinned, which is why this is a packet rather than a one-line fix: `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` asserts exactly one code-1003 warning at value 3 (`interface_bottom_layers_emits_one_typed_diagnostic`) and zero at `-1`/absent (`interface_bottom_layers_default_emits_no_typed_diagnostic`). Both tests must be rewritten to the implemented contract; neither may be weakened or deleted.

**The design blocker, and why it is the packet's real work.** `PlannedSupportNode` carries `x`, `y`, `dist_to_top` and `to_buildplate`. There is no `dist_to_bottom` and no record of where a branch lands on model geometry — exactly what a bottom band needs. That absence is structural, not an oversight: `plan_for_object` walks layers top→bottom in a single pass, nodes carry no parent pointers, and a chain's landing layer is unknown until the chain terminates, which is *after* every one of its nodes has already been emitted. A `dist_to_bottom` field therefore cannot be filled during the walk. Implementing only the `< 0 ⇒ top` fallback would delete an honest warning and still emit zero geometry.

The resolution this packet adopts is to move the computation out of node space entirely: a post-pass over the already-emitted `SupportPlanEntry` rows, run after `smooth_branches`, that reuses the existing per-layer `LayerCollisionCache` to ask canonical's own question — *is there model surface directly below where this chain stopped?* — and densifies upward from there. See `design.md` §Code Change Surface for the mechanism and its one accepted approximation.

This is one coherent slice: the fallback resolution, the landing detection, the band emission, the stub retirement, and the two test rewrites are all mutually dependent — landing the fallback without the geometry is the failure mode `DEV-129` explicitly warns against.

No packet is reopened or superseded. Packet 116 removed the dead Rust state; packet 118 owned the code-1003 record. This packet retires that record because the underlying gap is closed.

## Architecture Constraints

- **The blocker and its resolution.** `PlannedSupportNode` cannot carry `dist_to_bottom`. `plan_for_object` walks layers top→bottom once, nodes hold no parent pointers, and a chain's landing layer is only known after the chain terminates — by which time every node in it has already been emitted into `entries_in_order`. Any `dist_to_bottom` field would be `None` at emission time for every node that matters. The computation therefore moves out of node space into a post-pass over the emitted `SupportPlanEntry` rows, where the chain is fully materialised and the per-layer `LayerCollisionCache` is still in scope. **This packet adds no field to `PlannedSupportNode`**; that is a deliberate design outcome, not an omission.
- **The landing test, and its one accepted approximation.** Canonical `TreeSupport::draw_circles` searches *every* object layer below a support component for `stTop`/`stBottom` surfaces intersecting it, and treats the lowest such intersection as the true support-to-model contact. PnP's prepass has no surface classification — `SupportGeometryView.entries[..].outlines` is the coarse per-layer model footprint and nothing more. PnP therefore asks the reduced question: *does the chain's lowest emitted layer `L_end` have model footprint directly beneath it at `L_end - 1`?* This is correct whenever a chain stops because it hit the model (the code-1002 drop condition is literally "the moved target is inside `collision_polys`"), and it correctly answers "no" for a chain that reaches layer 0 on open build plate. It differs from canonical for a chain that stops for a different reason — the per-layer branch cap — directly above model geometry; such a chain gets a band canonical would not draw. Record this on the `DEV-129` closure row; do not silently ship it as parity.
- **Ordering is load-bearing.** `densify_bottom_interface` runs **after** `smooth_branches`, so the band is centred on the position that will actually be printed. The pre-existing top-interface densification runs *before* smoothing, inside the layer loop; that asymmetry is real and is left alone here (`[FWD-2]`).
- **Chains, not columns.** `group_branches_into_columns` groups by `(object_id, region_id)`; one region routinely contains several independent trees, which is exactly why `smooth_branches` already splits a column into sub-chains at XY gaps above `CHAIN_BREAK_THRESHOLD`. The bottom pass must use the same decomposition or it will attach one floor band to whichever tree happens to reach lowest. The split is therefore extracted into a shared helper rather than duplicated — duplicating it would recreate `DEV-127`'s "two drifted copies" failure mode inside a single file.
- <!-- snippet: wasm-staleness -->
- Guest WASM is **not** rebuilt by `cargo build` or `cargo test`. After editing any path in this packet's change surface that feeds the guest build (see `CLAUDE.md` §"Guest WASM Staleness"), the implementer MUST run `cargo xtask build-guests --check` and, if `STALE:` is reported, rebuild without `--check` before re-running the failing test. Stale-guest failures look unrelated to the change but are caused by it.
- <!-- snippet: coord-system -->
- Coordinate units: **1 unit = 100 nm** (10⁻⁴ mm), NOT 1 nm like OrcaSlicer. Divide OrcaSlicer constants by 100. Use `Point2::from_mm(x, y)` or `mm_to_units()` at every mm↔unit boundary. Full porting checklist in `docs/08_coordinate_system.md`.
- Concretely: no canonical constant is transcribed by this packet — `support_interface_bottom_layers` is a layer **count**, dimensionless. The band's spatial extent reuses the top band's own `radius + tree_support_branch_distance * 0.5` half-extent and `tree_support_interface_spacing_mm`, both already-resolved PnP config values. If the implementer is ever tempted to lift a `scale_(...)` literal from `TreeSupportCommon.hpp` for the band geometry, it is 100× too large.
- No schema or public version constant is bumped. `support-planner.toml` loses a comment only; every `[config.schema.*]` value is byte-identical, so the generated key table in `docs/15_config_keys_reference.md` does not move and no regeneration step is required.
- ADR conformance: `docs/adr/0010-typed-diagnostic-channel.md`'s normative Decision is "add a typed `Diagnostic` record to the prepass world". Retiring one of its three example call sites does not contradict that decision, so this packet **conforms** rather than amends: no `D-211-ADR-0010-AMENDED` deviation is needed. Only the ADR's descriptive §Status paragraph gains a retirement sentence.

## Data and Contract Notes

- IR/manifest contracts: `SupportPlanEntry`, `RaftPlan` and `Diagnostic` are unchanged. `support-planner.toml`'s `[config.schema.support_interface_bottom_layers]` keeps `type = "int"`, `default = -1`, `min = -1`, `max = 10`, `display`, `group`; only a preceding comment line is deleted. No config key is added or retyped.
- WIT boundary: unchanged. Bottom-band geometry travels as ordinary `branch_segments` in the existing `SupportPlanEntry`, exactly as the top band does. No WIT file is edited, so the `CLAUDE.md` WIT/Type-Changes checklist does not fire — but the guest-staleness rule does, for both `src/**` and the manifest.
- Determinism/scheduler constraints: the post-pass is a deterministic walk over `group_branches_into_columns`' `BTreeMap`-ordered output and appends to `branch_segments` in a fixed order. No claim, stage, or dependency edge changes. One diagnostic code (1003) stops being emitted; `ModuleAccessAudit.diagnostics` is a `Vec` with no expected length, so no host-side assertion moves.

## Locked Assumptions and Invariants

- **Locked (behaviour change):** `support_interface_bottom_layers = -1` — the shipped default — now resolves to `support_interface_top_layers` (default 2) and therefore produces bottom bands on every default slice with model-landing supports. This is canonical (`number_of_support_interface_bottom_layers`) and `CLAUDE.md` puts canonical parity above baseline stability. Reversible only by changing the manifest default, which would itself be a divergence.
- **Locked:** `0` means "no floor band" and never falls back to the top count. Only `< 0` triggers the fallback.
- **Locked:** bands attach only where a chain lands on model geometry, and never when `support_on_build_plate_only` is true.
- **Locked:** the band extends **upward** from the landing layer, occupying `[L_end, L_end + bottom_n)`. It never reaches below `L_end`.
- **Invariant:** the bottom band uses the same scan-line emitter, half-extent formula, parity rule and avoidance/collision clipping as the top band. If the two ever diverge, that is a defect, not a feature.
- **Not locked:** the exact `densify_bottom_interface` parameter list. It is private; the implementer may pass a small context struct instead of eight scalars if clippy's `too_many_arguments` fires (the existing `push_interface_scan_lines` carries an `#[allow]` for precisely this).

## Risks and Tradeoffs

- **Default output moves.** Every default slice with model-landing supports gains two densified layers. `cargo test -p slicer-runtime --test integration support_invariants_wedge_tdd` and any G-code baseline that includes tree supports may shift. Baselines must be re-recorded to the canonical-correct output, never the behaviour reverted to keep them green (`CLAUDE.md` §Test Discipline).
- **The cap-truncation false positive.** A chain truncated by `max_branches_per_layer` directly above model geometry gets a band canonical would not draw. Bounded and documented; it produces slightly more interface, never less, and never places geometry inside the model.
- **The chain-split extraction is the sneaky risk.** It touches `smooth_branches`, which is shipped and tested. Landing it as its own step with `smooth_nodes_tdd` as the gate (Step 2) is what keeps a later band failure attributable.
- **Segment-count assertions are coarse.** The ACs compare `branch_segments.len()` between two runs of one fixture. This proves a band appeared where expected and nowhere else, but not its exact line placement. That is the honest limit of an IR with no interface/base marker; strengthening it means the IR change rejected above.
- **Clippy `too_many_arguments`.** `push_interface_scan_lines` already carries `#[allow(clippy::too_many_arguments)]`; `densify_bottom_interface` will likely need the same or a context struct. Budgeted in Step 4.
