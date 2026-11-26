---
status: draft
packet: 294-flush-into-purge-reuse-wipe-tower
task_ids: []
backlog_source: docs/specs/orca-feature-gap/issues/72-author-packet-p65-multimaterial-flush-options-tool-ordering.md (wayfinder map: Close the OrcaSlicer FFF feature gap)
context_cost_estimate: M
copy_note: Queue packet from the wayfinder map "Close the OrcaSlicer FFF feature gap"; authored under ticket 72 (P65).
---

# Packet Contract: 294-flush-into-purge-reuse-wipe-tower

## Goal

Make the P65 flush-into trio drive purge reuse at parity with canonical `WipingExtrusions` — `flush_into_infill` / `flush_into_objects` / `flush_into_support` as scalar-global gates over which already-printed new-tool roles absorb purge volume — ported as one wiping-volume subtraction inside `wipe-tower`'s `run_finalization` path, with no new module, IR field, or WIT change.

## Scope Boundaries

P65 is three Tier B keys whose canonical behaviour is purge accounting in `ToolOrdering.cpp`, not ordering: marked extrusions print with the incoming tool and each decrements the remaining tower purge. Claim-time grounding (ticket 72) holds all three in: each is live in canonical's slicing pipeline (no dead key, no alias) and zero-occurrence as behaviour in this tree (no `crates/`/`modules/`/`xtask/` read, no `ORCA_CONFIG_PADDING` row, no prior packet). The tier table's `tool-ordering` owner is corrected to `wipe-tower` (ticket-27/39/40 precedent): this tree's ordering stage (`PathOptimizationDefault::group_then_nearest_neighbor`) ignores config entirely and owns only sequence, which flush-into does not change — the purge decision point (`WipeTower::purge_volume_for` → `purge_depth_for` → `generate_purge_paths`, emitted by `run_finalization`) is wipe-tower's. The packet declares the three bools on `wipe-tower.toml` (canonical defaults `false`/`false`/`true`), computes per-toolchange wiping volume from the layer's own post-anchor new-tool entities, and subtracts it from that toolchange's purge (floored at zero). Canonical per-object shape is a recorded simplification (DEV-186(a)); canonical's filament-assignment gating is a recorded non-borrow (DEV-186(b)); order is untouched (DEV-186(d)).

## Prerequisites and Blockers

- Depends on: nothing. All symbols below are live on HEAD (verified at authoring); ticket 30's `purge_volume_for` is landed tree code, not a packet dependency.
- Related work, not a blocker: ticket 122 (prime tower body) — purge-only tower suffices here (less purge = fewer scan lines from the existing generator), so no sequencing edge; ticket 125 (per-tool model) — explicitly NOT this packet's axis (per-tool/extruder vectors vs per-object scalars are different dimensions; DEV-186(a)).
- Unblocks: wayfinder ticket 72 (P65 closes with all three keys in). No edge to any other draft packet.
- Activation blockers: none. DEV-186 is the next collision-free ID (LOG max DEV-171; drafts claim DEV-172–DEV-185 — re-derive `max(DEV-*)` over `docs/DEVIATION_LOG.md` + `docs/spec_packets/*/` before writing the row).

## Acceptance Criteria

- **AC-1. Given** default config, **when** the wipe-tower schema is inspected, **then** all three keys are declared bool with canonical defaults — `flush_into_infill = false`, `flush_into_objects = false`, `flush_into_support = true` — and `WipeTower::from_config` reads each (a `true`/`true`/`false` config flips all three fields). | `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd schema_declares_flush_into_keys 2>&1 | tee target/test-output.log | tail -5`
- **AC-2. Given** default config (support gate `true`) with a toolchange layer carrying only wall + sparse entities and no support entities, **when** `run_finalization` emits, **then** tower inserts are identical to the all-flags-false baseline (the `true` default is inert without its subject); and with support entities of measured volume `V_s` present, the emitted purge depth equals `(45.0 − V_s) / (line_width × layer_height × tower_width)` within `1e-3`. | `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd default_support_gate_is_subject_gated 2>&1 | tee target/test-output.log | tail -5`
- **AC-3. Given** `flush_into_infill = true` with post-anchor new-tool sparse entities of measured volume `V`, **when** `run_finalization` emits, **then** that toolchange's purge depth equals `(45.0 − V) / cross_section` within `1e-3`; and with only wall entities present the depth is unchanged from baseline (infill flag opens sparse only). | `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd infill_flag_absorbs_sparse_only 2>&1 | tee target/test-output.log | tail -5`
- **AC-4. Given** `flush_into_objects = true` with post-anchor wall + solid entities of measured volume `V`, **when** `run_finalization` emits, **then** that toolchange's purge depth equals `(45.0 − V) / cross_section` within `1e-3` (walls and solids count; support roles do not unless the support flag is also set). | `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd objects_flag_absorbs_walls_and_solids 2>&1 | tee target/test-output.log | tail -5`
- **AC-5. Given** `flush_into_support = false` with support entities present, **when** `run_finalization` emits, **then** the purge depth equals the all-flags-false baseline exactly (support entities ignored); the `true`→`false` flip is the support key's non-default behaviour pin. | `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd support_false_ignores_support_entities 2>&1 | tee target/test-output.log | tail -5`

## Negative Test Cases

- **AC-N1. Given** the authored tree, **when** the CONFIG_BLOCK padding table is inspected, **then** no `flush_into_*` row exists in `ORCA_CONFIG_PADDING` and the table is untouched by this packet (honest absence per the packet-260/261 precedent — module keys with no host twin). | `rg -q 'flush_into' crates/slicer-gcode/src/serialize.rs && exit 1 || exit 0`
- **AC-N2. Given** `flush_into_objects = true` with post-anchor bridge entities (`BridgeInfill`, `InternalBridgeInfill`) present, **when** `run_finalization` emits, **then** the purge depth equals baseline (bridge roles never count — DEV-186(c)). | `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd bridge_roles_never_count 2>&1 | tee target/test-output.log | tail -5`

## Verification

- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test -p wipe-tower --test flush_into_purge_reuse_tdd 2>&1 | tee target/test-output.log | tail -5`

## Authoritative Docs

- `docs/00_project_overview.md` - delegated SUMMARY (modular pipeline + community extensibility constraint on the wipe-tower-seam shape)
- `docs/01_system_architecture.md` - delegated SUMMARY (Claim System section only — three bools parameterise one module over roles it already sees; rule 4 trigger test does not fire: in-module purge parameter, not cross-module algorithm selection)
- `docs/03_wit_and_manifest.md` - delegated SUMMARY (manifest `[config.schema]` bool-row shape + `from_declared` whitelist section only — declaration is required for the read to fire, ticket-34 lesson)

## Doc Impact Statement (Required)

- `docs/DEVIATION_LOG.md` section "DEV-186" - `rg -q 'DEV-186' docs/DEVIATION_LOG.md` (filed in Step 4 with (a)+(b)+(c)+(d) clauses)
- `docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` section "Multimaterial / Flush options" - `rg -q 'flush_into_infill.*wipe-tower' docs/specs/orca-feature-gap/issues/04-asset-tier-assignment.md` (Step 4 owner correction)
- `docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` section "P65" - `rg -q '294-flush-into' docs/specs/orca-feature-gap/issues/05-asset-packet-list.md` (Step 4: 3 keys in at packet 294)

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

<!-- snippet: context-discipline -->
## Context Discipline Note

This packet was generated against the context_discipline preamble shared by `spec-packet-generator`, `swarm`, and `spec-review`. Downstream agents implementing or reviewing this packet must:

- treat `design.md`'s code change surface as the authoritative files-in-scope list
- honor `design.md`'s out-of-bounds list — those files must not be loaded directly
- delegate every cargo run and authoritative-doc fact-check
- obey the shared absolute context bands: 120k reading budget with hand-off at 150k (standard); the extended band (240k reading / 300k hard stop) only via swarm's escalation protocol

Aggregate context cost above is the sum of per-step costs in `implementation-plan.md`. If any single step is rated L, the packet must be split before activation (an extended-band run may carry a single L step only when `design.md` justifies why it cannot be split).
