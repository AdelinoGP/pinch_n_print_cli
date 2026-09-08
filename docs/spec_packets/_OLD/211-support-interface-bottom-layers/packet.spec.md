---
status: superseded
packet: 211-support-interface-bottom-layers
task_ids:
  - TASK-327
backlog_source: docs/07_implementation_status.md
context_cost_estimate: M
superseded_by: 210a-support-planner-coord-t + 210b-support-interface-bottom-layers
superseded_on: 2026-08-07
---

# Packet Contract: 211-support-interface-bottom-layers

> **SUPERSEDED 2026-08-07 by user decision — absorbed into the merged `210`, which was then
> re-split the same day into `docs/spec_packets/210a-support-planner-coord-t/` (DEV-128, the
> coord_t migration + the `split_column_into_chains` extraction) and
> `docs/spec_packets/210b-support-interface-bottom-layers/` (DEV-129, this packet's successor).**
>
> **DO NOT IMPLEMENT THIS PACKET.** It is retained for provenance only. The merged `210`
> directory no longer exists; every path reference below that names it is historical.
>
> Do not implement this packet. `DEV-129` and this packet's `TASK-327` work are owned
> by packet 210, which now covers both `DEV-128` and `DEV-129`; `TASK-327` is folded
> into `TASK-326` and must not be registered separately in
> `docs/07_implementation_status.md`.
>
> **Why.** Both packets rewrote `smooth_branches`
> (`modules/core-modules/support-planner/src/lib.rs`) and neither plan accounted for
> the other's edit: packet 210 retyped the inlined sub-chain gap walk to an integer
> Laplacian keyed on `CHAIN_BREAK_THRESHOLD_UNITS`, while this packet extracted that
> same walk verbatim into `split_column_into_chains`. Applied in either order, the
> second edit deletes or duplicates the first. This packet's code surface was
> additionally written against pre-210 `f32` shapes (`first_point_xyw`,
> `push_interface_scan_lines`) despite declaring 210 as a dependency.
>
> This directory is retained for provenance only. It is frozen: never edit it, and
> never delete it. The merge rationale, the five corrections applied to the scope
> carried over from here, and the current authoritative contract are in
> `docs/spec_packets/210a-support-planner-coord-t/requirements.md` (historical reference: this named the merged `210` at the time of writing)
> §"Absorption of packet 211". Everything below this banner is historical.

## Goal

Replace `support_interface_bottom_layers`' warn-only code-1003 stub in `support-planner` with real bottom-interface (floor) bands: resolve the canonical `< 0 ⇒ use the top count` fallback, detect where each smoothed branch chain lands on model geometry below, and densify the lowest N layers of every model-landing chain with the same scan-line fill the top-interface band already uses.

## Scope Boundaries

In scope: `modules/core-modules/support-planner/src/lib.rs` (a new `SupportPlanner.support_interface_bottom_layers` field, the net-new `resolve_interface_bottom_layers` fallback, a chain-splitting helper extracted from `smooth_branches`, and a post-smoothing `densify_bottom_interface` pass), the module manifest's stale "Not yet implemented" comment, and the two `diagnostics_tdd.rs` tests that pin the warn-only contract. Out of scope: the WIT/IR wire format, the support **top**-interface band, the bottom **gap** (`support_bottom_z_distance`, which PnP has no key for), the `num_bottom_base_interface_layers` base/interface split canonical layers on top of the floor band, and every other core module.

## Prerequisites and Blockers

- Depends on: `210a-support-planner-coord-t` (was `210-support-planner-coord-t`, since re-split) — **FORWARD-DEP on a `draft` packet. HISTORICAL — the guidance in this bullet is superseded; `210b` owns the live version and requires `210a` to be IMPLEMENTED and merged, never implemented against pre-migration `f32` signatures.** Packet 210 retypes `PlannedSupportNode` to `x: i64, y: i64` and `push_interface_scan_lines` to a `Point2` / `i64` signature. This packet's `densify_bottom_interface` calls `push_interface_scan_lines` and reads node positions, so it must be implemented against 210's post-migration signatures. If 210 has not landed when this packet is activated, implement against the pre-migration `f32` signatures and re-verify after 210 merges — do not fork the file.
- Unblocks: nothing in the 206–212 queue.
- Activation blockers: none of substance. The one behaviour decision (default `-1` now produces bands, because it resolves to the top count) is locked in `design.md` §Locked Assumptions on canonical-parity grounds and is recorded in `[FWD-1]` if a user wishes to revisit it.

## Acceptance Criteria

- **AC-1. Given** the canonical fallback `number_of_support_interface_bottom_layers` (`SupportParameters.hpp`), **when** the net-new `pub fn resolve_interface_bottom_layers(bottom_layers: i32, top_layers: i32) -> u32` is exercised, **then** it returns `top_layers.max(0) as u32` for any `bottom_layers < 0` (both `-1` and `-5`), `3` for `bottom_layers == 3`, and `0` for `bottom_layers == 0` — i.e. negative means "same as top", never "disabled". | `cargo test -p support-planner --lib resolve_interface_bottom_layers 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo AC1-PASS`
- **AC-2. Given** a branch chain whose lowest emitted layer `L_end` has model collision geometry directly beneath it at layer `L_end - 1`, **when** the planner runs with `support_interface_bottom_layers = 3`, **then** the `SupportPlanEntry` rows at global layers `L_end`, `L_end + 1` and `L_end + 2` each carry strictly more `branch_segments` than the identical fixture run with `support_interface_bottom_layers = 0`. | `cargo test -p support-planner --test interface_bottom_layers_tdd bottom_band_densifies_three_layers_above_model_landing 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo AC2-PASS`
- **AC-3. Given** a branch chain that reaches global layer 0 with no model geometry beneath it, **when** the planner runs with `support_interface_bottom_layers = 3`, **then** every one of that chain's entries carries exactly the same `branch_segments` count as the `= 0` run — a build-plate landing gets no floor band, matching canonical's `found_contact == false` path in `TreeSupport::draw_circles`. | `cargo test -p support-planner --test interface_bottom_layers_tdd buildplate_landing_gets_no_bottom_band 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo AC3-PASS`
- **AC-4. Given** `support_on_build_plate_only = true`, **when** the planner runs with `support_interface_bottom_layers = 3`, **then** no bottom band is emitted anywhere — the whole pass is skipped, mirroring canonical's `!m_object_config->support_on_build_plate_only.value` guard on the floor-area block. | `cargo test -p support-planner --test interface_bottom_layers_tdd buildplate_only_disables_bottom_band 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo AC4-PASS`
- **AC-5. Given** the default configuration, **when** `support_interface_bottom_layers` is absent or `-1` and `support_interface_top_layers = 2`, **then** exactly 2 layers above each model landing are densified — the resolved count equals the top count, not zero. | `cargo test -p support-planner --test interface_bottom_layers_tdd default_minus_one_resolves_to_top_layer_count 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo AC5-PASS`
- **AC-6. Given** `support_interface_bottom_layers = 0`, **when** the planner runs, **then** no entry's `branch_segments` count differs from a run of the same fixture with the bottom pass compiled out — `0` is the explicit "no floor band" value and must not fall back to the top count. | `cargo test -p support-planner --test interface_bottom_layers_tdd zero_bottom_layers_emits_no_band 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo AC6-PASS`
- **AC-7. Given** the module manifest, **when** `[config.schema.support_interface_bottom_layers]` is inspected, **then** the preceding `# Not yet implemented — see docs/specs/support-modules-orca-port.md` comment is gone while `type = "int"`, `default = -1`, `min = -1` and `max = 10` are unchanged (the generated key table in `docs/15_config_keys_reference.md` therefore does not move). | `! rg -q 'Not yet implemented' modules/core-modules/support-planner/support-planner.toml && rg -qU '\[config.schema.support_interface_bottom_layers\][^\[]*default = -1' modules/core-modules/support-planner/support-planner.toml && echo AC7-PASS`
- **AC-8. Given** the wedge support invariant suite, **when** it runs with bottom bands enabled by default, **then** `branch_endpoints_are_outside_support_collision_outlines`, `branch_points_match_entry_layer_z`, `branch_radii_stay_within_current_bounds` and `support_plan_has_finite_branch_paths` all still pass — floor bands must not place geometry inside the model or off its layer's Z. | `cargo test -p slicer-runtime --test integration support_invariants_wedge_tdd 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo AC8-PASS`
- **AC-9. Given** the retired stub, **when** `modules/core-modules/support-planner/src/lib.rs` is searched, **then** neither the literal `1003` nor the string `is not yet implemented` appears anywhere in it. | `! rg -q 'code: 1003' modules/core-modules/support-planner/src/lib.rs && ! rg -q 'is not yet implemented' modules/core-modules/support-planner/src/lib.rs && echo AC9-PASS`

## Negative Test Cases

- **AC-N1. Given** `support_interface_bottom_layers = 3` — the exact input the retired stub warned on — **when** the planner runs, **then** **zero** code-1003 diagnostics are emitted, and the rewritten `diagnostics_tdd.rs` case asserts that count while still asserting the code-1001 and code-1002 contracts are untouched. The pre-existing `interface_bottom_layers_emits_one_typed_diagnostic` is rewritten to this new contract; it is not deleted and its "exactly N code-1003 records" shape is preserved with N = 0. | `cargo test -p support-planner --test diagnostics_tdd 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo ACN1-PASS`
- **AC-N2. Given** a model-landing chain with `support_interface_bottom_layers = 3`, **when** the entries at global layers **below** `L_end` are inspected, **then** none of them gained any `branch_segments` — the band extends upward from the landing, never downward into the model. | `cargo test -p support-planner --test interface_bottom_layers_tdd bottom_band_never_extends_below_landing_layer 2>&1 \| rg -q 'test result: ok\. [1-9][0-9]* passed; 0 failed' && echo ACN2-PASS`
- **AC-N3. Given** the guest-WASM build inputs changed by this packet (`modules/core-modules/support-planner/src/**` and `.../support-planner.toml`), **when** `cargo xtask build-guests --check` runs after the last edit, **then** it reports no `STALE:` line — a stale `support-planner.wasm` makes AC-8 measure the pre-change planner. | `cargo xtask build-guests --check 2>&1 \| rg -q 'STALE:' && echo FAIL \|\| echo PASS`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p support-planner --test interface_bottom_layers_tdd`

## Authoritative Docs

- `docs/15_config_keys_reference.md` (875 lines) — ranged read of the `**Note — support_interface_bottom_layers:**` paragraph that immediately follows the `<!-- END GENERATED: module-config-keys -->` marker. That paragraph is rewritten by this packet. Never read the file in full.
- `docs/adr/0010-typed-diagnostic-channel.md` (125 lines) — direct read of §Status only; its *normative* Decision (add a typed `Diagnostic` record to the prepass world) is unaffected, but its Status paragraph lists code 1003 as a shipped call site and must gain a retirement sentence.
- `docs/DEVIATION_LOG.md` — row `DEV-129` only (grep, then read that row); it is this packet's problem statement, carries the design blocker, and must be moved to `Closed`.
- `docs/07_implementation_status.md` (412 lines) — delegated `LOCATIONS` for the Workstream 3 insertion point and for the `TASK-163b-diagnostic` row that records code 1003 as shipped. Never read in full.

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/Support/SupportParameters.hpp` — `number_of_support_interface_bottom_layers`: the exact `< 0 ⇒ use the top count` fallback this packet ports, plus the `std::max(0, ...)` clamp applied where `num_bottom_interface_layers` is assigned.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupportCommon.hpp` — the `support_bottom_enable` / `support_bottom_height` / `support_floor_enable` / `support_floor_layers` assignments derived from that same call; establishes that "bottom interface" and "floor" are the same band under two names.
- `OrcaSlicerDocumented/src/libslic3r/Support/TreeSupport.cpp` — `TreeSupport::draw_circles`' floor-area block: the downward search for the true support-to-model contact surface, the `found_contact == false ⇒ no floor band` path, the `!support_on_build_plate_only` guard, and the "N interface layers above the contact" rule. This is the mechanism PnP approximates; the approximation is stated in `design.md`.
- `OrcaSlicerDocumented/src/libslic3r/Support/SupportCommon.cpp` — how the resulting floor areas are turned into interface extrusions, consulted only to confirm PnP's scan-line densification is the right analogue and that no separate role/flow is required.

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).

## Doc Impact Statement (Required)

Specific same-packet doc edits:

- `docs/15_config_keys_reference.md`, the `support_interface_bottom_layers` note following `<!-- END GENERATED: module-config-keys -->` — rewritten to state the implemented semantics (`-1` resolves to `support_interface_top_layers`; `0` disables; bands attach only where a branch lands on model geometry and only when `support_on_build_plate_only` is false) and to drop the dead `docs/specs/_OLD/support-modules-orca-port.md` pointer and the code-1003 sentence. Verification grep: `rg -q 'resolves to .support_interface_top_layers' docs/15_config_keys_reference.md && ! rg -q 'code-.1003' docs/15_config_keys_reference.md`
- `docs/adr/0010-typed-diagnostic-channel.md` §Status — one appended sentence recording that code 1003 was retired by this packet and that codes 1001/1002 are unaffected. The ADR's Decision section is **not** edited: retiring one call site does not contradict the decision to have a typed channel, so no ADR amendment deviation is required. Verification grep: `rg -q '1003.*retired' docs/adr/0010-typed-diagnostic-channel.md`
- `docs/DEVIATION_LOG.md` row `DEV-129` — status moved from `Open` to `Closed`, recording both the implementation and the accepted approximation (PnP tests the model footprint at `L_end - 1` where canonical searches all layers below for `stTop`/`stBottom` surfaces). Verification grep: `rg -q '^\| DEV-129 .*Closed' docs/DEVIATION_LOG.md`
- `docs/07_implementation_status.md` — new `TASK-327` row under §"Workstream 3 — Benchy parity and missing OrcaSlicer behavior", plus an amendment note on the existing `TASK-163b-diagnostic` row that code 1003 has been retired. Verification grep: `rg -q 'TASK-327' docs/07_implementation_status.md`
