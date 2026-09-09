# Requirements: 294-flush-into-purge-reuse-wipe-tower

## Packet Metadata

- Grouped task IDs: `TASK-000`
- Backlog source: `docs/specs/orca-feature-gap/issues/72-author-packet-p65-multimaterial-flush-options-tool-ordering.md`
- Packet status: `draft`
- Aggregate context cost: `M` (never L)

## Problem Statement

P65 (Multimaterial / Flush options, tool-ordering) is three Tier B keys whose canonical behaviour is purge accounting in `ToolOrdering.cpp` — marked extrusions print with the incoming tool and each decrements the remaining tower purge — but this port marks nothing and decrements nothing: `WipeTower::purge_volume_for` computes the pair's purge from matrix/fallback/grab-length alone, and `run_finalization` emits the full depth at every toolchange. All three keys are true zero-occurrence gaps (no behaviour reads outside map prose, no `ORCA_CONFIG_PADDING` row, no prior packet), and they form one coherent slice: three bools over the same layer view feeding one subtraction in the same function. Authoring fewer would split one subtraction; folding in neighbours (P23's already-landed matrix pair, P43's priming tower, ticket 122's body census) would repeat the mixed-seam failure the map's Authoring rule 1 prohibits.

## In Scope

- Declare `flush_into_infill` (bool, default `false`), `flush_into_objects` (bool, default `false`), and `flush_into_support` (bool, default `true`) in `wipe-tower.toml` `[config.schema]` (canonical defaults; no `min`/`max` — bools carry none), and read all three in `WipeTower::from_config` into three fields (canonical-false defaults inert; the `true`-default support key is inert without its subject, AC-2). Module declaration is required for the read to fire (`ConfigView::from_declared` whitelist, ticket-34 lesson). No `ResolvedConfig` field, no `to_config_map` arm, no host-keys row — wipe-tower reads its own manifest directly (ticket-30 precedent).
- Build the wiping-volume computation: for each toolchange `tc` in `run_finalization`, sum `entity_volume(entity)` over `view.ordered_entities()` entities positioned after `tc.after_entity_index` whose `tool_index == tc.to_tool` and whose role passes the flag gate — objects-flag opens every role except the support pair and `WipeTower`/`PrimeTower` self-roles; else infill-flag opens `SparseInfill` only; support-flag (independent disjunct) opens `SupportMaterial`/`SupportInterface`/`SupportBaseInterface`; `BridgeInfill`/`InternalBridgeInfill` never count (DEV-186(c)). Volumes use `length × width × layer_height` (the same formula both prior artifact-uses share — ticket-30's artifact and the estimator), and the helper lives beside `purge_volume_for` so the formula cannot drift.
- Subtract per toolchange inside the finalization path (`purge_depth_for` gains the layer-view + flags parameters, or a new `purged_depth_for` wrapper; `max_purge_depth` threads the same view): `depth = max(0, purge_volume_for(from,to) − wiping_volume(tc)) / cross_section`. The grab-length clamp's `.max(0.0)` extends to the subtraction (over-subscription yields no tower, never negative depth). Bed-bounds validation (`max_purge_depth`) automatically shrinks with the emission — same helper, cannot drift (AC-2's depth pins both).
- Scalar-global porting (DEV-186(a)): canonical declares the three keys per-object (`PrintObjectConfig`, consulted via `object.config()`), resolved per-object in `mark_wiping_extrusions`; the port's finalization seam sees one global `ConfigView` (ordering ignores config; `resolve_per_object_configs` exists in `crates/slicer-scheduler/src/config_resolution.rs` but is unthreaded on this path), so one value governs every layer. Accepted as the recorded simplification; a per-object future stays with the object-config axis, explicitly not ticket 125 (per-tool/extruder vectors are a different dimension).
- Annotate the 04 tier table + 05 packet list: P65 3 keys in, owner corrected `tool-ordering` → `wipe-tower`, packet number 294.

## Out of Scope

- Per-object resolution of the three flags: canonical walks `mark_wiping_extrusions` per object config; the port has no object identity at the `run_finalization` seam (`RegionKey.object_id` exists on entities but no per-object config is threaded there). Named simplification, not a silent omission (DEV-186(a)). A future that threads `resolve_per_object_configs` onto the finalization path stays out of scope.
- Canonical's filament-assignment gating (`is_support_overriddable`'s `support_filament == 0` conditions; `is_overriddable`'s soluble-filament veto): the finalization seam sees roles and tool indices, not `support_filament`/`support_interface_filament` assignments or solubility — roles are the only subject signal. Named non-borrow (DEV-186(b)); ticket 38's live filament routing is a different seam and is not consulted.
- Canonical's entity re-sorting (`ensure_perimeters_infills_order` + the ascending tool-cluster order): the port's order comes from `PathOptimizationDefault::group_then_nearest_neighbor` before finalization runs, and the tower inserts after anchors without moving existing entities. Order is untouched (DEV-186(d)) — this packet is purge accounting, not sequence.
- Reassigning entity `tool_index` to the incoming tool: canonical marks overridable extrusions with the incoming extruder before computing; the emitted G-code is identical either way here because purge entities already carry `tc.to_tool` and this packet emits no entity — it shrinks tower geometry. Named non-borrow (DEV-186(d)).
- Per-entity E reduction (shrinking the marked entities' own extrusion): canonical does not do this either — marked extrusions print whole with the new tool; only the tower shrinks. Not ported because canonical does not do it.
- `ORCA_CONFIG_PADDING` edits of any kind: zero twins exist for all three spellings (verified at authoring) and module keys without a host twin are honestly absent (packet-260/261 precedent). AC-N1 pins the absence.
- Cache invalidation (`Print::invalidate_state_by_config_options` / `PrintObject::invalidate_state_by_config_options`): the port recomputes `run_finalization` from live config on every slice with no purge cache, so there is nothing to invalidate. Named non-borrow, not a silent omission.

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline: new decision points go in the existing owner, not host special cases)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section: rule-4 trigger test — this subtraction is an in-module purge parameter, not cross-module algorithm selection, so no claim holders)
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest `[config.schema]` bool-row shape + the `from_declared` whitelist that makes declaration mandatory)

<!-- snippet: orca-delegation -->
## OrcaSlicer Reference Obligations

All OrcaSlicer reads MUST be delegated to a sub-agent. Never load `OrcaSlicerDocumented/` into the implementer's own context. Dispatch contract: return `LOCATIONS` (file:line + 1-line context, ≤ 20 entries) or `SUMMARY` (≤ 200 words, no code unless asked). Code snippets in returns are capped at 30 lines.

Files to inspect for this packet:

- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.hpp` — `PrintObjectConfig` declares the three scalar `ConfigOptionBool` keys (borrow the per-object shape note + canonical defaults infill `false` / objects `false` / support `true`; port scalar-global, DEV-186(a))
- `OrcaSlicerDocumented/src/libslic3r/PrintConfig.cpp` — `PrintConfigDef::init_fff_params` (the three `coBool` declarations + prime-tower-enabled tooltip prerequisite; borrow the enable-gating shape — the port's `!enabled` early return already matches)
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `WipingExtrusions::is_overriddable` (borrow the role gate: objects-flag opens any entity, else infill-flag opens only `erInternalInfill`; port the role table, DEV-186(c))
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `WipingExtrusions::is_support_overriddable` (borrow the support gate including its `support_filament` conditions; port bool-only — filament assignment is invisible at the finalization seam, DEV-186(b))
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `WipingExtrusions::mark_wiping_extrusions` (borrow the walk shape — purge objects first, infill pass, perimeter pass iff objects-flag, support pass iff support-flag — and the purge-decrement accounting; port volume-subtraction only, no re-sort, DEV-186(d))
- `OrcaSlicerDocumented/src/libslic3r/GCode/ToolOrdering.cpp` — `WipingExtrusions::ensure_perimeters_infills_order` (named non-borrow — the port never reorders; cite as the order-untouched evidence for DEV-186(d))
- `OrcaSlicerDocumented/src/libslic3r/Print.cpp` + `OrcaSlicerDocumented/src/libslic3r/PrintObject.cpp` — `invalidate_state_by_config_options` (named non-borrow — the port recomputes `run_finalization` per slice from live config with no purge cache; cite as the no-invalidation evidence)

<!-- snippet: parity-evidence -->
## Parity Evidence Standard

Every key this packet implements carries evidence per the map's ticket 02 standard:

- **Canonical read + described behaviour.** For each key, cite the canonical consumer (file + function, never line numbers) and describe its behaviour in `requirements.md`. Reads of `OrcaSlicerDocumented/` are delegated per the orca-delegation snippet.
- **Invariants, not goldens.** Behaviour is pinned with invariant/property tests (counts preserved, mappings hold, emitted values equal expected). Golden G-code comparison is not part of the standard — the checkout is not built and cannot be run.
- **Ported Orca tests are acceptable evidence.** When `OrcaSlicerDocumented/tests/fff_print/` covers the behaviour, port its assertions into PnP's suite with the standard porting header (`docs/ORCASLICER_ATTRIBUTION.md`).
- **Plumbing keys** (a threshold feeding an existing decision point): the default resolves to the canonical value AND a test proves the value reaches the consumer. No behavioural test required.
- **Unverifiable behaviour:** surface the key and the reason to the human first; only with their sign-off file a `docs/DEVIATION_LOG.md` row (single source of truth, CI-checked by `cargo xtask check-deviations`) and proceed with documented scope. Never defer the key or block the packet on unverifiability alone, and never file a row without the human having been asked.

## Acceptance Summary

Reference, never copy, criteria from `packet.spec.md`.

- Positive: `AC-1` (schema, all three keys canonical) through `AC-5` (support `false`-flip pin); refinements: AC-2 pins the `true`-default support key's subject-gating (inert without support entities — the honest-default invariant); AC-3 pins the infill flag's sparse-only qualification; AC-4 pins the objects flag's walls+solids absorption; each absorbed-volume assertion is measured against `(45.0 − V) / cross_section`, proving the subtraction sits behind the grab-length clamp, not beside it.
- Negative: `AC-N1` (honest CONFIG_BLOCK absence — no padding twin); `AC-N2` (bridge roles never count, DEV-186(c)).
- Cross-packet impact: default tower depth is subject-identical (walls+sparse carry no `true` flag at defaults; support entities shrink depth — intended, only when support co-occurs with a toolchange); bed-bounds validation shrinks with emission (same helper, AC-2 pins it); order untouched (no entity moves, DEV-186(d)); no host twin, no CONFIG_BLOCK change, no new command lines.

## Verification Commands

This is the authoritative full matrix; `packet.spec.md` lists only 2-3 gate commands.

| Command | Purpose | Return format hint |
| --- | --- | --- |
| `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd 2>&1 \| tee target/test-output.log \| tail -5` | Prove all ACs incl. schema/subject-gating/three flags/absence/bridge exclusion | FACT pass/fail; SNIPPETS <=20 lines on failure |
| `cargo check --workspace --all-targets 2>&1 \| tee target/test-output.log \| tail -3` | Prove no struct-literal or cross-crate breakage from the new fields | FACT pass/fail |
| `cargo clippy --workspace --all-targets -- -D warnings 2>&1 \| tee target/test-output.log \| tail -3` | Prove lint-clean finalization stage | FACT pass/fail |
| `cargo test -p wipe-tower 2>&1 \| tee target/test-output.log \| tail -3` | Prove no regression in the existing tower suite (purge/matrix/grab/bed-bounds) | FACT pass/fail |

Commands must have small, parseable output suitable for delegation.

## Step Completion Expectations

The three declarations land before their AC tests (Step 1); the subtraction lands with the gate in one step (Step 2) — splitting the volume helper from its call site would leave an untested helper (trap 1). `threshold`-style staging does not apply: there is no gate input beyond the three bools, and each bool's behaviour pin rides its own AC. Tier-table + packet-list annotation (Step 3) records P65 3-in with the owner correction.

## Context Discipline Notes

Packet-specific hazards: `modules/core-modules/wipe-tower/src/lib.rs` is over 600 lines — use ranged reads only (ranges in `design.md`); tempting full reads of `ToolOrdering.cpp` are out-of-bounds (delegate per the obligations above); the `WipeTower` struct-field addition carries struct-literal blast radius (owned by Step 1's dispatch, not discovered via follow-up check — the in-file `#[cfg(test)]` literals plus every `WipeTower { .. }` site); the `purge_depth_for` signature change carries call-site blast radius (`generate_purge_paths` is depth-adjacent but must NOT change — it keeps calling the un-subtracted helper; owned by Step 2's dispatch).
