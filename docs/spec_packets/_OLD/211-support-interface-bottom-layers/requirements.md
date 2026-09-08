# Requirements: 211-support-interface-bottom-layers

## Packet Metadata

- Grouped task IDs: `TASK-327` (net-new; re-derive that the slot is still free at the moment you register it)
- Backlog source: `docs/07_implementation_status.md`
- Packet status: `draft`
- Aggregate context cost: `M`

## Problem Statement

`DEV-129` (`docs/DEVIATION_LOG.md`) records that `support_interface_bottom_layers` is registered, parsed, and inert. `support-planner.toml` declares it under a `# Not yet implemented` comment with `default = -1`, and `SupportPlanner::run_support_geometry` reads it only to `push_diagnostic` a code-1003 `Warn` — `"…is not yet implemented (config value={interface_bottom_layers})"` — before the layer loop. No geometry is produced, so PnP's support interfaces are top-only. Canonical implements it: `number_of_support_interface_bottom_layers` (`SupportParameters.hpp`) applies a `< 0 ⇒ use the top count` fallback, `TreeSupportCommon.hpp` turns the result into `support_floor_enable` / `support_floor_layers`, and `TreeSupport::draw_circles` builds real `floor_areas` from it.

The warn-only state is deliberate and test-pinned, which is why this is a packet rather than a one-line fix: `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` asserts exactly one code-1003 warning at value 3 (`interface_bottom_layers_emits_one_typed_diagnostic`) and zero at `-1`/absent (`interface_bottom_layers_default_emits_no_typed_diagnostic`). Both tests must be rewritten to the implemented contract; neither may be weakened or deleted.

**The design blocker, and why it is the packet's real work.** `PlannedSupportNode` carries `x`, `y`, `dist_to_top` and `to_buildplate`. There is no `dist_to_bottom` and no record of where a branch lands on model geometry — exactly what a bottom band needs. That absence is structural, not an oversight: `plan_for_object` walks layers top→bottom in a single pass, nodes carry no parent pointers, and a chain's landing layer is unknown until the chain terminates, which is *after* every one of its nodes has already been emitted. A `dist_to_bottom` field therefore cannot be filled during the walk. Implementing only the `< 0 ⇒ top` fallback would delete an honest warning and still emit zero geometry.

The resolution this packet adopts is to move the computation out of node space entirely: a post-pass over the already-emitted `SupportPlanEntry` rows, run after `smooth_branches`, that reuses the existing per-layer `LayerCollisionCache` to ask canonical's own question — *is there model surface directly below where this chain stopped?* — and densifies upward from there. See `design.md` §Code Change Surface for the mechanism and its one accepted approximation.

This is one coherent slice: the fallback resolution, the landing detection, the band emission, the stub retirement, and the two test rewrites are all mutually dependent — landing the fallback without the geometry is the failure mode `DEV-129` explicitly warns against.

No packet is reopened or superseded. Packet 116 removed the dead Rust state; packet 118 owned the code-1003 record. This packet retires that record because the underlying gap is closed.

## In Scope

- Add `support_interface_bottom_layers: i32` to `SupportPlanner`, parsed in `from_config` alongside `support_interface_top_layers` (Int and Float accepted, default `-1`), so the value is available in `plan_for_object` via `self` rather than only from `run_support_geometry`'s `_config`.
- Add `pub fn resolve_interface_bottom_layers(bottom_layers: i32, top_layers: i32) -> u32` — the port of canonical `number_of_support_interface_bottom_layers`: `if bottom_layers < 0 { top_layers } else { bottom_layers }`, then `.max(0) as u32`. Public so AC-1 can pin it without a full planner run.
- Extract the sub-chain splitting currently inlined in `smooth_branches` (the `CHAIN_BREAK_THRESHOLD` gap walk that produces `sub_starts`) into a private helper shared by `smooth_branches` and the new bottom pass, so both agree on what a "branch chain" is. A region column can contain several independent trees; using the whole column would attach one floor band to the lowest tree only.
- Add a private `densify_bottom_interface` post-pass over `entries_in_order`, run in `plan_for_object` **after** `smooth_branches`, which for each sub-chain: takes the chain's lowest-layer entry `L_end`; classifies the landing as *on model* iff `L_end > 0` and the entry's smoothed XY lies inside `collision_polys` at `L_end - 1`; and, when on model, appends scan-line fill to the entries at `L_end .. L_end + bottom_n - 1` using the existing `push_interface_scan_lines`.
- Guard the whole pass on `!self.support_on_build_plate_only` and on `bottom_n > 0 && self.tree_support_interface_spacing_mm > 0.0`, mirroring the existing top-interface guard and canonical's floor-area guard.
- Thread `collision_cache` (already a `plan_for_object` parameter) into the post-pass; no new plumbing across `run_support_geometry`.
- Delete the code-1003 `push_diagnostic` block and its `interface_bottom_layers` local from `run_support_geometry`.
- Rewrite `interface_bottom_layers_emits_one_typed_diagnostic` and `interface_bottom_layers_default_emits_no_typed_diagnostic` in `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` to the new contract: zero code-1003 records in both cases, with the "exactly N" assertion shape preserved at N = 0 and the file's module doc header updated to say why.
- Add `modules/core-modules/support-planner/tests/interface_bottom_layers_tdd.rs` with the six band-behaviour cases named by AC-2 through AC-6 and AC-N2, built on the collision-cache fixture pattern already used by `unreachable_buildplate_node_pruned` in `tests/to_buildplate_tdd.rs` (per-layer `SupportGeometryViewEntry.outlines`, small footprint at the contact layer, large footprint below).
- Add the in-file `#[cfg(test)]` cases for `resolve_interface_bottom_layers` (AC-1).
- Remove the `# Not yet implemented — see docs/specs/support-modules-orca-port.md` comment above `[config.schema.support_interface_bottom_layers]` in `modules/core-modules/support-planner/support-planner.toml`, leaving `type`/`default`/`min`/`max`/`display`/`group` byte-identical.
- Update `docs/15_config_keys_reference.md`'s note paragraph, `docs/adr/0010-typed-diagnostic-channel.md` §Status, `docs/DEVIATION_LOG.md` row `DEV-129`, and `docs/07_implementation_status.md`.

## Out of Scope

- Any WIT, IR, or manifest **schema** change. No new config key, no retyped key, no new `Diagnostic` code, no `SupportPlanEntry` field. The generated key table in `docs/15_config_keys_reference.md` does not move because no schema value changes.
- The **bottom gap** (`bottom_gap_height` in canonical's floor block, driven by `slicing_params.gap_object_support`). PnP has no corresponding config key on `support-planner`; canonical's gap-clearing `diff_ex` has no analogue here. Bands attach directly at the landing layer.
- Canonical's `num_bottom_base_interface_layers` split, which reclassifies the lower part of the floor band as base rather than interface (`floor_interface_as_base`). PnP has no base/interface role distinction inside `SupportPlanEntry.branch_segments`.
- The **top**-interface band. Its `dist_to_top`-driven densification inside the layer loop is the model this packet mirrors, and it is read but not modified. Note the pre-existing asymmetry: the top band is emitted *before* `smooth_branches` and the bottom band *after*; correcting that ordering is a separate concern (see `design.md` `[FWD-2]`).
- Canonical's requirement that the contact surface be classified `stTop`/`stBottom`, and its search across **all** layers below rather than only the one immediately beneath. PnP's `SupportGeometryView.outlines` carries no surface classification; the approximation is documented in `design.md` and recorded on the `DEV-129` closure row.
- Codes 1001 (`max_branches_per_layer` cap) and 1002 (`node-clamped-out`), and every other assertion in `diagnostics_tdd.rs`. Only the two bottom-layers cases are rewritten.
- Packet 210's coordinate migration. This packet neither performs nor reverses it.

## Authoritative Docs

- `docs/15_config_keys_reference.md` — 875 lines; ranged read of the `support_interface_bottom_layers` note only, located by grep. Delegate anything wider.
- `docs/adr/0010-typed-diagnostic-channel.md` — 125 lines; direct read of §Status. Its §Decision is normative and untouched.
- `docs/DEVIATION_LOG.md` — large; grep `DEV-129` and read that row alone.
- `docs/07_implementation_status.md` — 412 lines; delegate `LOCATIONS` for the Workstream 3 insertion point and the `TASK-163b-diagnostic` row.
- `docs/20_support_preview.md` — delegated SUMMARY only, and only if the implementer needs to confirm that no preview surface distinguishes interface from base segments (it does not affect the design; skip unless a question arises).

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `number_of_support_interface_bottom_layers`: the exact `< 0 ⇒ use the top count` fallback this packet ports, plus the `std::max(0, ...)` clamp applied where `num_bottom_interface_layers` is assigned.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` — the `support_bottom_enable` / `support_bottom_height` / `support_floor_enable` / `support_floor_layers` assignments derived from that same call; establishes that "bottom interface" and "floor" are the same band under two names.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::draw_circles`' floor-area block: the downward search for the true support-to-model contact surface, the `found_contact == false ⇒ no floor band` path, the `!support_on_build_plate_only` guard, and the "N interface layers above the contact" rule. This is the mechanism PnP approximates; the approximation is stated in `design.md`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — how the resulting floor areas are turned into interface extrusions, consulted only to confirm PnP's scan-line densification is the right analogue and that no separate role/flow is required.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` through `AC-9`.
  - `AC-1` pins the canonical fallback on all four branches (`-1`, `-5`, `0`, `3`). The `-5` case matters: `< 0` means "same as top", so any negative must resolve to the top count, not clamp to zero.
  - `AC-2`/`AC-3`/`AC-4`/`AC-5`/`AC-6` are all differential — each compares `branch_segments` counts against the same fixture at a different setting. A differential assertion is the only cheap way to observe the band, because `SupportPlanEntry` carries no interface/base role marker; an absolute count would be brittle against unrelated planner changes.
  - `AC-8` is the regression guard the whole packet risks: bands are new geometry placed near the model, so invariant 2 (`branch_endpoints_are_outside_support_collision_outlines`) is the likeliest silent break.
  - `AC-9` proves the stub is actually gone rather than merely unreachable.
- Negative: `AC-N1` (zero code-1003 records at the exact value that used to warn), `AC-N2` (bands never extend below the landing layer, i.e. never into the model), `AC-N3` (guest freshness, without which AC-8 measures the old planner).
- Cross-packet impact: consumes `210-support-planner-coord-t`'s retyped `push_interface_scan_lines` and `PlannedSupportNode`. Produces `resolve_interface_bottom_layers` as the only net-new public symbol; nothing outside `modules/core-modules/support-planner/` consumes it today.

## Verification Commands

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p support-planner --lib resolve_interface_bottom_layers` | AC-1 fallback semantics | FACT pass/fail |
| `cargo test -p support-planner --test interface_bottom_layers_tdd` | AC-2..AC-6, AC-N2 — the whole band contract | FACT pass/fail + failing case names |
| `cargo test -p support-planner --test diagnostics_tdd` | AC-N1 — rewritten 1003 contract, codes 1001/1002 unchanged | FACT pass/fail |
| `cargo test -p support-planner --test to_buildplate_tdd` | `support_on_build_plate_only` and code-1002 drop behaviour unaffected | FACT pass/fail |
| `cargo test -p support-planner --test smooth_nodes_tdd` | Chain-splitting extraction did not change smoothing behaviour | FACT pass/fail |
| `cargo test -p support-planner` | Whole-crate sweep; `support-planner`'s `Cargo.toml` declares no `[features]` and no `required-features` targets, so this compiles every test binary | FACT pass/fail + `test result:` line count |
| `cargo xtask build-guests --check` | AC-N3; `src/**` and the manifest are guest inputs | FACT: reports `STALE:` yes/no |
| `cargo test -p slicer-runtime --test integration support_invariants_wedge_tdd` | AC-8 on the real wedge fixture, against a fresh guest | FACT pass/fail; SNIPPETS ≤20 lines on failure |
| `! rg -q 'Not yet implemented' modules/core-modules/support-planner/support-planner.toml` | AC-7 | FACT pass/fail |
| `rg -q 'TASK-327' docs/07_implementation_status.md` | Doc impact | FACT pass/fail |
| `rg -q '^\| DEV-129 .*Closed' docs/DEVIATION_LOG.md` | Doc impact | FACT pass/fail |
| `cargo check --workspace --all-targets` | Closure gate | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings` | Closure gate | FACT pass/fail + lint names |

## Step Completion Expectations

- The chain-splitting extraction (Step 2) must be behaviour-preserving on its own: `smooth_nodes_tdd` has to stay green *before* any bottom-band code exists, otherwise a later band failure cannot be attributed.
- `densify_bottom_interface` must run after `smooth_branches` in `plan_for_object`. Running it before means the band is centred on the unsmoothed node and then the smoother moves the structural point away from its own floor band. This ordering is load-bearing and is asserted indirectly by AC-8.
- `cargo xtask build-guests --check` must be run after the last `src/lib.rs` **or** `support-planner.toml` edit and before AC-8. Both paths are guest inputs.
- If `210-support-planner-coord-t` has not merged when this packet is implemented, `push_interface_scan_lines` still takes `f32` millimetre arguments. Implement against whichever signature is on disk, verify, and re-verify after 210 merges. Do not add a compatibility shim and do not edit 210's packet directory.
- `TASK-327`'s availability and the `DEV-129` row's current text are ledger facts; re-derive both at the moment of the Step 5 edit.

## Context Discipline Notes

- `modules/core-modules/support-planner/src/lib.rs` is ~2 060 lines. The only ranges this packet needs are: the `SupportPlanner` struct + `from_config`, the top-interface densification block and the `smooth_branches` call at the end of `plan_for_object`, the `group_branches_into_columns` / `smooth_branches` / `push_interface_scan_lines` helper block, and the `#[cfg(test)] mod tests` header. Four ranged reads; never open it in full.
- `modules/core-modules/support-planner/tests/diagnostics_tdd.rs` is 506 lines. Read only the two bottom-layers cases and the fixture helpers they call (`make_planner_config`, `make_layer_plan`, `make_region_segmentation`, `small_overhang_fixture`); the code-1001/1002 cases are out of bounds.
- `modules/core-modules/support-planner/tests/to_buildplate_tdd.rs` is 570 lines and is **read-only**. Read only `unreachable_buildplate_node_pruned` and the `multi_overhang_grid` / `make_layer_plan` helpers — that is the fixture shape the new test file copies. Do not read its other three cases.
- `docs/15_config_keys_reference.md` contains a large generated table. Grep for `support_interface_bottom_layers` and read the surrounding note paragraph only; never load the table.
- Canonical's floor-area block in `TreeSupport::draw_circles` is long and dense. Take it as a bounded SUMMARY answering three specific questions (what triggers a floor band, what happens when no contact is found, what disables it) — do not request the code.
